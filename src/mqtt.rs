use std::time::Duration;

use async_trait::async_trait;
use log::error;
use rumqttc::Transport;
use rumqttc::v5::{AsyncClient, EventLoop, MqttOptions};
use rumqttc::v5::mqttbytes::{qos, QoS};
use tokio::time;
use crate::config::Config;

use crate::publisher::Publisher;

pub struct Mqtt {
    client: AsyncClient,
    qos: QoS,
    retain: bool
}

impl Mqtt {
    pub fn new(config: &Config) -> (Box<dyn Publisher>, Box<EventLoop>) {
        let mut mqtt_options = MqttOptions::new(config.mqtt_client_id.clone(), config.mqtt_host.clone(), config.mqtt_port.clone());
        if config.mqtt_use_tls_transport {
            mqtt_options.set_transport(Transport::tls_with_default_config());
        }
        mqtt_options.set_keep_alive(Duration::from_secs(5));
        mqtt_options.set_clean_start(config.mqtt_clean_start);
        if let Some(username) = config.mqtt_username.clone() {
            if let Some(password) = config.mqtt_password.clone() {
                mqtt_options.set_credentials(username.to_owned(), password.to_owned());
            }
        }

        let (client, event_loop) = AsyncClient::new(mqtt_options, 10);
        (
            Box::new(Mqtt { client, qos: qos(config.mqtt_topic_qos).unwrap_or(QoS::AtMostOnce), retain: config.mqtt_topic_retain }),
            Box::new(event_loop),
        )
    }
}

#[async_trait]
impl Publisher for Mqtt {
    async fn publish(&mut self, topic: String, data: Vec<u8>) {
        for _ in 1..=30 {
            match self.client.publish(topic.to_owned(), self.qos, self.retain, data.clone()).await {
                Ok(_) => { return; }
                Err(err) => {
                    error!("Failed to publish message: {:?}", err);
                    time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
    //
    // async fn destroy(&self) {
    //     if let Err(e) = self.client.try_disconnect() {
    //         error!("Failed to try to disconnect mqtt client: {:?}", e);
    //     }
    //     if let Err(e) = self.client.disconnect().await {
    //         error!("Failed to disconnect mqtt client: {:?}", e);
    //     }
    // }
}
