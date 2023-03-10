use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct DeviceDiscoveredEvent {
    pub event: String,
    pub id: btleplug::platform::PeripheralId,
}

#[derive(Serialize, Deserialize)]
pub struct DeviceUpdatedEvent {
    pub event: String,
    pub id: btleplug::platform::PeripheralId,
}

#[derive(Serialize, Deserialize)]
pub struct DeviceConnectedEvent {
    pub event: String,
    pub id: btleplug::platform::PeripheralId,
}

#[derive(Serialize, Deserialize)]
pub struct DeviceDisconnectedEvent {
    pub event: String,
    pub id: btleplug::platform::PeripheralId,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct ManufacturerDataAdvertisementEvent {
    pub event: String,
    pub id: btleplug::platform::PeripheralId,
    pub manufacturer_data: HashMap<u16, String>,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct ServiceDataAdvertisementEvent {
    pub event: String,
    pub id: btleplug::platform::PeripheralId,
    pub service_data: HashMap<Uuid, String>,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct ServicesAdvertisementEvent {
   pub event: String,
   pub id: btleplug::platform::PeripheralId,
   pub services: Vec<Uuid>,
}
