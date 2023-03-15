use std::env;
use std::error::Error;
use std::pin::Pin;
use std::time::Duration;

use anyhow::Context;
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use envconfig::Envconfig;
use futures::FutureExt;
use futures::Stream;
use futures::stream::StreamExt;
use log::{debug, error, info, LevelFilter};
use rumqttc::v5::{AsyncClient, EventLoop, MqttOptions};
use rumqttc::v5::mqttbytes::QoS;
use serde_json::to_vec;
use tokio::{select, time};
use tokio::signal;

use config::Config;
use event::{*};

mod config;
mod event;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let verbose = env::args().any(|x| x.eq_ignore_ascii_case("-v") || x.eq_ignore_ascii_case("--verbose"));

    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .init();


    info!("Initializing configuration");
    let config = Config::init_from_env()?;

    let mut mqtt_options = MqttOptions::new(config.mqtt_client_id.clone(), config.mqtt_host.clone(), config.mqtt_port.clone());
    mqtt_options.set_keep_alive(Duration::from_secs(5));

    info!("Connecting to mqtt broker mqtt://{}:{} ..", config.mqtt_host.clone(), config.mqtt_port.clone());
    let (mqtt_client, mut event_loop) = AsyncClient::new(mqtt_options, 10);

    let adapter = get_adapter().await?;
    let mut events = adapter.events().await?;
    adapter.start_scan(ScanFilter::default()).await?;
    info!("Scanning for ble events..");

    info!("Processing events..");
    loop {
        select! {
            _ = process_ble_events(&config, &adapter, &mqtt_client, verbose, &mut events).fuse() => {}
            _ = process_mqtt_event_loop(&mut event_loop).fuse() => {}
            _ = process_ctrl_c().fuse() => {
                break
            }
        }
    }

    Ok(())
}

async fn process_ctrl_c() {
    if let Ok(_) = signal::ctrl_c().await {
        info!("Shutting down..");
        return;
    }
}

async fn process_mqtt_event_loop(event_loop: &mut EventLoop) {
    if let Err(err) = event_loop.poll().await {
        error!("event loop error: {:?}", err)
    }
}

async fn process_ble_events(config: &Config, adapter: &Adapter, mqtt_client: &AsyncClient, verbose: bool, events: &mut Pin<Box<dyn Stream<Item=CentralEvent> + Send>>) {
    if let Some(event) = events.next().await {
        match process_central_event(&config, &adapter, event, verbose).await {
            Ok((payload, topic_name)) => {
                debug!("topic: {}", &topic_name);

                publish_to_topic(&mqtt_client, topic_name, payload).await
            }
            Err(err) => error!("Failed to process central event: {:?}", err)
        }
    }
}

async fn get_adapter() -> anyhow::Result<Adapter> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    return Ok(adapters.into_iter().nth(0).context("no adapter")?);
}

async fn process_central_event(config: &Config, adapter: &Adapter, event: CentralEvent, verbose: bool) -> anyhow::Result<(Vec<u8>, String)> {
    let topic: String;
    let event = match event {
        CentralEvent::DeviceDiscovered(id) => {
            let (name, mac_address, rssi) = get_properties(adapter, &id).await?;
            topic = format!("{}/{}/{}", config.mqtt_topic, "DeviceDiscovered", name.clone().unwrap_or(id.to_string()));
            Event::new(id.to_string(), "DeviceDiscovered".into(), mac_address, name, rssi, None, None, None)
        }
        CentralEvent::DeviceUpdated(id) => {
            let (name, mac_address, rssi) = get_properties(adapter, &id).await?;
            topic = format!("{}/{}/{}", config.mqtt_topic, "DeviceUpdated", name.clone().unwrap_or(id.to_string()));
            Event::new(id.to_string(), "DeviceUpdated".into(), mac_address, name, rssi, None, None, None)
        }
        CentralEvent::DeviceConnected(id) => {
            let (name, mac_address, rssi) = get_properties(adapter, &id).await?;
            topic = format!("{}/{}/{}", config.mqtt_topic, "DeviceConnected", name.clone().unwrap_or(id.to_string()));
            Event::new(id.to_string(), "DeviceConnected".into(), mac_address, name, rssi, None, None, None)
        }
        CentralEvent::DeviceDisconnected(id) => {
            let (name, mac_address, rssi) = get_properties(adapter, &id).await?;
            topic = format!("{}/{}/{}", config.mqtt_topic, "DeviceDisconnected", name.clone().unwrap_or(id.to_string()));
            Event::new(id.to_string(), "DeviceDisconnected".into(), mac_address, name, rssi, None, None, None)
        }
        CentralEvent::ManufacturerDataAdvertisement { id, manufacturer_data } => {
            let data = manufacturer_data.iter().
                map(|(k, v)| (k.clone(), hex::encode(v))).
                collect();
            let (name, mac_address, rssi) = get_properties(adapter, &id).await?;
            topic = format!("{}/{}/{}", config.mqtt_topic, "ManufacturerDataAdvertisement", name.clone().unwrap_or(id.to_string()));

            if verbose {
                let peripheral = adapter.peripheral(&id).await?;
                let props = peripheral.properties().await?.unwrap();
                info!("ManufacturerDataAdvertisement: peripheral: {}, address: {}, local_name: {:?}, rssi: {:?}, tx_power_level: {:?} manufacturer_data: {:?}", peripheral.id().to_string(),  props.address, props.local_name, props.rssi, props.tx_power_level, manufacturer_data);
            }

            Event::new(id.to_string(), "ManufacturerDataAdvertisement".into(), mac_address, name, rssi, Some(data), None, None)
        }
        CentralEvent::ServiceDataAdvertisement { id, service_data } => {
            let (name, mac_address, rssi) = get_properties(adapter, &id).await?;
            topic = format!("{}/{}/{}", config.mqtt_topic, "ServiceDataAdvertisement", name.clone().unwrap_or(id.to_string()));
            let data = service_data.iter().
                map(|(k, v)| (k.clone(), hex::encode(v))).
                collect();

            if verbose {
                let peripheral = adapter.peripheral(&id).await?;
                let props = peripheral.properties().await?.unwrap();
                info!("ServiceDataAdvertisement: peripheral: {}, address: {}, local_name: {:?}, rssi: {:?}, tx_power_level: {:?} service_data: {:?}", peripheral.id().to_string(),  props.address, props.local_name, props.rssi, props.tx_power_level, service_data);
            }

            Event::new(id.to_string(), "ServiceDataAdvertisement".into(), mac_address, name, rssi, None, Some(data), None)
        }
        CentralEvent::ServicesAdvertisement { id, services } => {
            let (name, mac_address, rssi) = get_properties(adapter, &id).await?;
            topic = format!("{}/{}/{}", config.mqtt_topic, "ServicesAdvertisement", name.clone().unwrap_or(id.to_string()));

            if verbose {
                let peripheral = adapter.peripheral(&id).await?;
                let props = peripheral.properties().await?.unwrap();
                info!("ServicesAdvertisement: peripheral: {}, address: {}, local_name: {:?}, rssi: {:?}, tx_power_level: {:?} services: {:?}", peripheral.id().to_string(),  props.address, props.local_name, props.rssi, props.tx_power_level, services);
            }

            Event::new(id.to_string(), "ServicesAdvertisement".into(), mac_address, name, rssi, None, None, Some(services))
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

async fn publish_to_topic(mqtt_client: &AsyncClient, topic: String, payload: Vec<u8>) {
    for _ in 1..=30 {
        match mqtt_client.publish(topic.to_owned(), QoS::AtMostOnce, false, payload.clone()).await {
            Ok(_) => { return; }
            Err(err) => {
                error!("Failed to publish message: {:?}", err);
                time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
