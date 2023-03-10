use std::error::Error;
use std::io::Write;
use std::time::Duration;

use anyhow::Context;
use btleplug::api::{Central, CentralEvent, Manager as _, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use chrono::Local;
use envconfig::Envconfig;
use futures::stream::StreamExt;
use log::{error, info, LevelFilter, warn};
use mqtt::properties;
use paho_mqtt as mqtt;
use paho_mqtt::{AsyncClient, Topic};
use paho_mqtt::Error::PahoDescr;
use tokio::time;
use config::Config;
use events::{*};

mod config;
mod events;

const MQTT_CLIENT_DISCONNECTED: i32 = -3;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}] {}: {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter_level(LevelFilter::Info)
        .init();


    let config = Config::init_from_env()?;

    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(format!("mqtt://{}:{}", config.mqtt_host, config.mqtt_port))
        .client_id(config.mqtt_client_id.unwrap_or_default())
        .finalize();

    let mqtt_client = AsyncClient::new(create_opts).expect("Error creating client");

    let props = properties! {
        mqtt::PropertyCode::SessionExpiryInterval => 86400,
    };

    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(config.mqtt_keep_alive_interval_seconds))
        .clean_start(config.mqtt_clean_start)
        .properties(props)
        .user_name(config.mqtt_username.unwrap_or_default())
        .password(config.mqtt_password.unwrap_or_default())
        .finalize();

    info!("Connecting to mqtt broker..");
    mqtt_client.connect(conn_opts).wait().expect("Error connecting to mqtt broker");
    info!("Connected to mqtt broker");

    let topic = Topic::new(&mqtt_client, config.mqtt_topic, config.mqtt_topic_qos.unwrap_or_default());

    let adapter = get_adapter().await?;
    let mut events = adapter.events().await?;
    adapter.start_scan(ScanFilter::default()).await?;
    info!("Scanning for ble events..");

    while let Some(event) = events.next().await {
        let payload: Vec<u8>;
        match event {
            CentralEvent::DeviceDiscovered(id) => {
                payload = serde_json::to_vec(&DeviceDiscoveredEvent { event: "DeviceDiscovered".into(), id })?;
            }
            CentralEvent::DeviceUpdated(id) => {
                payload = serde_json::to_vec(&DeviceUpdatedEvent { event: "DeviceUpdated".into(), id })?;
            }
            CentralEvent::DeviceConnected(id) => {
                payload = serde_json::to_vec(&DeviceConnectedEvent { event: "DeviceConnected".into(), id })?;
            }
            CentralEvent::DeviceDisconnected(id) => {
                payload = serde_json::to_vec(&DeviceDisconnectedEvent { event: "DeviceDisconnected".into(), id })?;
            }
            CentralEvent::ManufacturerDataAdvertisement { id, manufacturer_data } => {
                let data = manufacturer_data.
                    iter().
                    map(|(k, v)| (k.clone(), hex::encode(v))).
                    collect();
                payload = serde_json::to_vec(&ManufacturerDataAdvertisementEvent { event: "ManufacturerDataAdvertisement".into(), id, manufacturer_data: data })?;
            }
            CentralEvent::ServiceDataAdvertisement { id, service_data } => {
                let data = service_data.
                    iter().
                    map(|(k, v)| (k.clone(), hex::encode(v))).
                    collect();
                payload = serde_json::to_vec(&ServiceDataAdvertisementEvent { event: "ServiceDataAdvertisement".into(), id, service_data: data })?;
            }
            CentralEvent::ServicesAdvertisement { id, services } => {
                payload = serde_json::to_vec(&ServicesAdvertisementEvent { event: "ServicesAdvertisement".into(), id, services })?;
            }
        }

        publish_to_topic(&mqtt_client, &topic, &payload).await
    }

    Ok(())
}

async fn publish_to_topic<'a>(mqtt_client: &AsyncClient, topic: &Topic<'a>, payload: &Vec<u8>) {
    for _ in 1..=30 {
        let mut reconnected = false;
        match topic.publish(payload.to_owned()).await {
            Ok(_) => { return; }
            Err(err) => match err {
                PahoDescr(id, reason) => {
                    if id == MQTT_CLIENT_DISCONNECTED {
                        error!("Failed to publish message id: {:?} reason: {:?}", id, reason);
                        continue;
                    }

                    warn!("Connection to the mqtt broker was lost, attempting to reconnect..");
                    match mqtt_client.reconnect().await {
                        Ok(_) => {
                            info!("Reconnected");
                            reconnected = true;
                        }
                        Err(reconnect_err) => error!("Failed to reconnect: {:?}", reconnect_err),
                    }
                }
                _ => error!("Failed to publish message: {:?}", err),
            }
        }
        if !reconnected {
            time::sleep(Duration::from_secs(1)).await;
        }
    }
}

async fn get_adapter() -> anyhow::Result<Adapter> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    return Ok(adapters.into_iter().nth(0).context("no adapter")?);
}
