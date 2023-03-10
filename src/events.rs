use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Event<T> where T: Serialize {
    pub event: String,
    pub id: btleplug::platform::PeripheralId,
    pub data: T,
}

impl<T> Event<T> where T: Serialize {
    pub(crate) fn new(event: String, id: btleplug::platform::PeripheralId, data: T) -> Self {
        Self {
            event,
            id,
            data,
        }
    }
}

impl Event<Option<String>> {
    pub(crate) fn new_simple(event: String, id: btleplug::platform::PeripheralId) -> Self {
        Self {
            event,
            id,
            data: None,
        }
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct ManufacturerDataAdvertisementEvent {
    pub manufacturer_data: HashMap<u16, String>,
}

impl ManufacturerDataAdvertisementEvent {
    pub(crate) fn new(id: btleplug::platform::PeripheralId, manufacturer_data: HashMap<u16, String>) -> Event<Self> {
        Event::new("ManufacturerDataAdvertisementEvent".into(), id, ManufacturerDataAdvertisementEvent { manufacturer_data })
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct ServiceDataAdvertisementEvent {
    pub service_data: HashMap<Uuid, String>,
}

impl ServiceDataAdvertisementEvent {
    pub(crate) fn new(id: btleplug::platform::PeripheralId, service_data: HashMap<Uuid, String>) -> Event<Self> {
        Event::new("ServiceDataAdvertisementEvent".into(), id, ServiceDataAdvertisementEvent { service_data })
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct ServicesAdvertisementEvent {
    pub services: Vec<Uuid>,
}

impl ServicesAdvertisementEvent {
    pub(crate) fn new(id: btleplug::platform::PeripheralId, services: Vec<Uuid>) -> Event<Self> {
        Event::new("ServicesAdvertisement".into(), id, ServicesAdvertisementEvent { services })
    }
}
