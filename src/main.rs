use std::collections::HashMap;
use envconfig::Envconfig;
use btleplug::api::{Central, CentralEvent, Manager as _, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use uuid::Uuid;
use std::error::Error;
use std::time::Duration;
use anyhow::Context;
use futures::stream::StreamExt;
use mqtt::properties;
use paho_mqtt as mqtt;
use paho_mqtt::{AsyncClient, Topic};
use paho_mqtt::Error::PahoDescr;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tokio::time;

#[derive(Envconfig)]
struct Config {
    #[envconfig(from = "MQTT_HOST")]
    pub mqtt_host: String,

    #[envconfig(from = "MQTT_PORT", default = "1883")]
    pub mqtt_port: u16,

    #[envconfig(from = "MQTT_USERNAME")]
    pub mqtt_username: Option<String>,

    #[envconfig(from = "MQTT_PASSWORD")]
    pub mqtt_password: Option<String>,

    #[envconfig(from = "MQTT_CLIENT_ID")]
    pub mqtt_client_id: Option<String>,

    #[envconfig(from = "MQTT_TOPIC")]
    pub mqtt_topic: String,

    #[envconfig(from = "MQTT_TOPIC_QOS")]
    pub mqtt_topic_qos: Option<i32>,
}

#[derive(Serialize, Deserialize)]
struct DeviceDiscoveredEvent {
    event: String,
    id: btleplug::platform::PeripheralId,
}

#[derive(Serialize, Deserialize)]
struct DeviceUpdatedEvent {
    event: String,
    id: btleplug::platform::PeripheralId,
}


#[derive(Serialize, Deserialize)]
struct DeviceConnectedEvent {
    event: String,
    id: btleplug::platform::PeripheralId,
}

#[derive(Serialize, Deserialize)]
struct DeviceDisconnectedEvent {
    event: String,
    id: btleplug::platform::PeripheralId,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
struct ManufacturerDataAdvertisementEvent {
    event: String,
    id: btleplug::platform::PeripheralId,
    manufacturer_data: HashMap<u16, String>,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
struct ServiceDataAdvertisementEvent {
    event: String,
    id: btleplug::platform::PeripheralId,
    service_data: HashMap<Uuid, String>,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
struct ServicesAdvertisementEvent {
    event: String,
    id: btleplug::platform::PeripheralId,
    services: Vec<Uuid>,
}

const MQTTCLIENT_DISCONNECTED: i32 = -3;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::init_from_env()?;

    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(format!("mqtt://{}:{}", config.mqtt_host, config.mqtt_port))
        .client_id(config.mqtt_client_id.unwrap_or_default())
        .finalize();

    let mqtt_client = mqtt::AsyncClient::new(create_opts).expect("Error creating client");

    // Session will exist for a day (86,400 sec) between connections.
    let props = properties! {
        mqtt::PropertyCode::SessionExpiryInterval => 86400,
    };

    // Connect with MQTT v5 and a persistent server session (no clean start).
    // For a persistent v5 session, we must set the Session Expiry Interval
    // on the server. Here we set that requests will persist for a day
    // (86,400sec) if the service disconnects or restarts.
    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(20))
        .clean_start(false)
        .properties(props)
        .user_name(config.mqtt_username.unwrap_or_default())
        .password(config.mqtt_password.unwrap_or_default())
        .finalize();

    println!("Connecting to mqtt broker..");
    mqtt_client.connect(conn_opts).wait().expect("Error connecting to mqtt broker");
    println!("Connected to mqtt broker");

    let topic = mqtt::Topic::new(&mqtt_client, config.mqtt_topic, config.mqtt_topic_qos.unwrap_or_default());

    let manager = Manager::new().await?;
    let central = get_central(&manager).await?;
    let mut events = central.events().await?;
    central.start_scan(ScanFilter::default()).await?;
    println!("Scanning for ble events..");

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
                    if id == MQTTCLIENT_DISCONNECTED {
                        println!("Failed to publish message id: {:?} reason: {:?}", id, reason);
                        continue;
                    }

                    println!("Connection to the mqtt broker was lost, attempting to reconnect..");
                    match mqtt_client.reconnect().await {
                        Ok(_) => {
                            println!("Reconnected");
                            reconnected = true;
                        }
                        Err(reconnect_err) => println!("Failed to reconnect: {:?}", reconnect_err),
                    }
                }
                _ => println!("Failed to publish message: {:?}", err),
            }
        }
        if !reconnected {
            time::sleep(Duration::from_secs(1)).await;
        }
    }
}

async fn get_central(manager: &Manager) -> anyhow::Result<Adapter> {
    let adapters = manager.adapters().await?;
    return Ok(adapters.into_iter().nth(0).context("no adapter")?);
}
