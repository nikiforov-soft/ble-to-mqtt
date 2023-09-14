use std::time::Duration;

use async_trait::async_trait;
use log::error;
use memphis_rust_community::memphis_client::MemphisClient;
use memphis_rust_community::producer::{ComposableMessage, MemphisProducer, MemphisProducerOptions};
use memphis_rust_community::station::MemphisStationsOptions;
use tokio::time;

use crate::publisher::Publisher;

pub struct Memphis {
    producer: MemphisProducer,
}

impl Memphis {
    pub async fn new(
        hostname: String,
        username: String,
        password: String,
        station: String,
        producer_name: String,
    ) -> anyhow::Result<Box<dyn Publisher>> {
        let client = MemphisClient::new(hostname.as_str(), username.as_str(), password.as_str(), None).await?;
        let station_options = MemphisStationsOptions::new(station.as_str());
        let station = client.create_station(station_options).await?;

        let producer_options = MemphisProducerOptions::new(producer_name.as_str());
        let producer = station.create_producer(producer_options).await?;

        Ok(Box::new(Memphis { producer }))
    }
}

#[async_trait]
impl Publisher for Memphis {
    async fn publish(&mut self, topic: String, data: Vec<u8>) {
        for _ in 1..=30 {
            match self.producer.produce(
                ComposableMessage::new()
                    .with_payload(data.clone())
                    .with_header("path", topic.as_str())
            ).await {
                Ok(_) => { return; }
                Err(err) => {
                    error!("Failed to publish message: {:?}", err);
                    time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    // async fn destroy(&self) {
    //     // if let Err(e) = self.producer.destroy().await {
    //     //     error!("Failed to destroy producer: {:?}", e);
    //     // }
    // }
}
