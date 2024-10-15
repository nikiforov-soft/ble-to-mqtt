use std::env;
use std::path::Path;
use std::pin::Pin;

use anyhow::Context;
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use envconfig::Envconfig;
use futures::FutureExt;
use futures::Stream;
use futures::stream::StreamExt;
use log::{debug, error, info, LevelFilter};
use rumqttc::v5::EventLoop;
use serde_json::to_vec;
use tokio::select;
use tokio::signal;

use config::Config;
use event::{*};

use crate::memphis::Memphis;
use crate::mqtt::Mqtt;
use crate::publisher::Publisher;

mod config;
mod event;
mod mqtt;
mod publisher;
mod memphis;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let verbose = env::args().any(|x| x.eq_ignore_ascii_case("-v") || x.eq_ignore_ascii_case("--verbose"));

    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .init();

    info!("Initializing configuration");
    let config = Config::init_from_env()?;

    let topic_name: String;
    let mut publisher: Box<dyn Publisher>;
    let mut event_loop: Option<Box<EventLoop>>;
    if config.mqtt_enabled {
        info!("Connecting to mqtt broker on mqtt://{}:{} ..", config.mqtt_host.clone(), config.mqtt_port.clone());
        let (p, e) = Mqtt::new(&config);
        publisher = p;
        event_loop = Some(e);
        topic_name = config.mqtt_topic.clone()
    } else if config.memphis_enabled {
        info!("Connecting to memphis broker on {} ..", config.memphis_hostname.clone());
        publisher = Memphis::new(
            config.memphis_hostname.clone(),
            config.memphis_username.clone(),
            config.memphis_password.clone(),
            config.memphis_station.clone(),
            config.memphis_producer_name.clone(),
        ).await?;
        event_loop = None;
        topic_name = config.memphis_station.clone()
    } else {
        anyhow::bail!("enable mqtt or memphis")
    }

    info!("Publishing events on topic: {}", config.mqtt_topic.clone());

    let adapter = init_ble_adapter().await?;
    let mut events = adapter.events().await?;
    adapter.start_scan(ScanFilter::default()).await?;
    info!("Scanning for ble events..");

    info!("Processing events..");
    let mut ble_scan_restart_interval = time::interval(time::Duration::from_secs(config.bt_auto_scan_restart_interval_seconds));
    loop {
        select! {
            _ = &mut Box::pin(ble_scan_restart_interval.tick()) => {
                if let Err(err) = adapter.stop_scan().await {
                    error!("failed to stop scanner: {:?}", err)
                }
                if let Err(err) = adapter.start_scan(ScanFilter::default()).await {
                    error!("failed to start scanner: {:?}", err)
                }
                info!("Restarting bt scanner")
            },
            _ = process_ble_events(topic_name.clone(), &adapter, &mut publisher, verbose, &mut events).fuse() => {}
            _ = process_mqtt_event_loop(&mut event_loop).fuse() => {}
            _ = process_ctrl_c().fuse() => {
                break;
            }
        }
    }

    adapter.stop_scan().await?;
    publisher.destroy().await;

    Ok(())
}

async fn init_ble_adapter() -> anyhow::Result<Adapter> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    info!("Found {} bluetooth adapters", adapters.len());

    Ok(adapters.into_iter().nth(0).context("no adapter")?)
}

async fn process_ctrl_c() {
    if let Ok(_) = signal::ctrl_c().await {
        info!("Shutting down..");
        return;
    }
}

async fn process_ble_events(topic_name: String, adapter: &Adapter, publisher: &mut Box<dyn Publisher>, verbose: bool, events: &mut Pin<Box<dyn Stream<Item=CentralEvent> + Send>>) {
    if let Some(event) = events.next().await {
        match process_central_event(topic_name, &adapter, event, verbose).await {
            Ok((payload, topic_name)) => {
                debug!("topic: {}", &topic_name);

                publisher.publish(topic_name, payload).await
            }
            Err(err) => error!("Failed to process central event: {:?}", err)
        }
    }
}

async fn process_mqtt_event_loop(event_loop: &mut Option<Box<EventLoop>>) {
    if let Some(event_loop) = event_loop {
        if let Err(err) = event_loop.poll().await {
            error!("event loop error: {:?}", err)
        }
    } else {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await
    }
}

async fn process_central_event(topic_name: String, adapter: &Adapter, event: CentralEvent, verbose: bool) -> anyhow::Result<(Vec<u8>, String)> {
    let topic: String;
    let event = match event {
        CentralEvent::DeviceDiscovered(id) => {
            let (name, mac_address, rssi) = get_properties(adapter, &id).await?;
            topic = format_topic("DeviceDiscovered".into(), topic_name, adapter, Some(&id)).await?;
            Event::new(id.to_string(), "DeviceDiscovered".into(), mac_address, name, rssi, None, None, None)
        }
        CentralEvent::DeviceUpdated(id) => {
            let (name, mac_address, rssi) = get_properties(adapter, &id).await?;
            topic = format_topic("DeviceUpdated".into(), topic_name, adapter, Some(&id)).await?;
            Event::new(id.to_string(), "DeviceUpdated".into(), mac_address, name, rssi, None, None, None)
        }
        CentralEvent::DeviceConnected(id) => {
            let (name, mac_address, rssi) = get_properties(adapter, &id).await?;
            topic = format_topic("DeviceConnected".into(), topic_name, adapter, Some(&id)).await?;
            Event::new(id.to_string(), "DeviceConnected".into(), mac_address, name, rssi, None, None, None)
        }
        CentralEvent::DeviceDisconnected(id) => {
            let (name, mac_address, rssi) = get_properties(adapter, &id).await?;
            topic = format_topic("DeviceDisconnected".into(), topic_name, adapter, Some(&id)).await?;
            Event::new(id.to_string(), "DeviceDisconnected".into(), mac_address, name, rssi, None, None, None)
        }
        CentralEvent::ManufacturerDataAdvertisement { id, manufacturer_data } => {
            let data = manufacturer_data.iter().
                map(|(k, v)| (k.clone(), hex::encode(v))).
                collect();
            let (name, mac_address, rssi) = get_properties(adapter, &id).await?;
            topic = format_topic("ManufacturerDataAdvertisement".into(), topic_name, adapter, Some(&id)).await?;

            if verbose {
                let peripheral = adapter.peripheral(&id).await?;
                let props = peripheral.properties().await?.unwrap();
                info!("ManufacturerDataAdvertisement: topic: {},  rssi: {:?}, tx_power_level: {:?} manufacturer_data: {:?}", topic.clone(),  props.rssi, props.tx_power_level, manufacturer_data);
            }

            Event::new(id.to_string(), "ManufacturerDataAdvertisement".into(), mac_address, name, rssi, Some(data), None, None)
        }
        CentralEvent::ServiceDataAdvertisement { id, service_data } => {
            let (name, mac_address, rssi) = get_properties(adapter, &id).await?;
            topic = format_topic("ServiceDataAdvertisement".into(), topic_name, adapter, Some(&id)).await?;
            let data = service_data.iter().
                map(|(k, v)| (k.clone(), hex::encode(v))).
                collect();

            if verbose {
                let peripheral = adapter.peripheral(&id).await?;
                let props = peripheral.properties().await?.unwrap();
                info!("ServiceDataAdvertisement: topic: {}, rssi: {:?}, tx_power_level: {:?} service_data: {:?}", topic.clone(),  props.rssi, props.tx_power_level, service_data);
            }

            Event::new(id.to_string(), "ServiceDataAdvertisement".into(), mac_address, name, rssi, None, Some(data), None)
        }
        CentralEvent::ServicesAdvertisement { id, services } => {
            let (name, mac_address, rssi) = get_properties(adapter, &id).await?;
            topic = format_topic("ServicesAdvertisement".into(), topic_name, adapter, Some(&id)).await?;

            if verbose {
                let peripheral = adapter.peripheral(&id).await?;
                let props = peripheral.properties().await?.unwrap();
                info!("ServicesAdvertisement: topic: {}, rssi: {:?}, tx_power_level: {:?} services: {:?}", topic.clone(),  props.rssi, props.tx_power_level, services);
            }

            Event::new(id.to_string(), "ServicesAdvertisement".into(), mac_address, name, rssi, None, None, Some(services))
        }
        CentralEvent::StateUpdate(state) => {
            topic = format_topic(format!("StateUpdate/{:?}", state).into(), topic_name, adapter, None).await?;

            if verbose {
                info!("StateUpdate: state: {:?}", state);
            }

            Event::new("".into(), "StateUpdate".into(), "".into(), None, None, None, None, None)
        }
    };
    let payload = to_vec(&event)?;
    Ok((payload, topic))
}

async fn get_properties(adapter: &Adapter, id: &btleplug::platform::PeripheralId) -> anyhow::Result<(Option<String>, String, Option<i16>)> {
    let peripheral = adapter.peripheral(&id).await?;
    if let Some(properties) = peripheral.properties().await? {
        return Ok((properties.local_name, properties.address.to_string(), properties.rssi));
    }
    Ok((None, peripheral.address().to_string(), None))
}

async fn format_topic(kind: String, topic: String, adapter: &Adapter, id: Option<&btleplug::platform::PeripheralId>) -> anyhow::Result<String> {
    let mut p = Path::new(topic.clone().as_str()).join(kind);
    if let Some(id) = id {
        p = p.join(id.to_string().replace("/", "_"));

        let peripheral = adapter.peripheral(&id).await?;
        if let Some(properties) = peripheral.properties().await? {
            if let Some(name) = properties.local_name {
                p = p.join(name);
            }
        }
    }

    return Ok(p.display().to_string());
}
