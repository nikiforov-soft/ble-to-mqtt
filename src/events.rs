use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Event<T> where T: Serialize {
    pub event: String,
    pub local_name: Option<String>,
    pub rssi: Option<i16>,
    pub id: String,
    pub data: T,
}

impl<T> Event<T> where T: Serialize {
    pub(crate) fn new(event: String, local_name: Option<String>, rssi: Option<i16>, id: String, data: T) -> Self {
        Self {
            event,
            local_name,
            rssi,
            id,
            data,
        }
    }
}

impl Event<Option<String>> {
    pub(crate) fn new_simple(event: String, local_name: Option<String>, rssi: Option<i16>, id: String) -> Self {
        Self {
            event,
            local_name,
            rssi,
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
    pub(crate) fn new(
        local_name: Option<String>,
        rssi: Option<i16>,
        id: String,
        manufacturer_data: HashMap<u16, String>
    ) -> Event<Self> {
        Event::new("ManufacturerDataAdvertisementEvent".into(), local_name, rssi, id, ManufacturerDataAdvertisementEvent { manufacturer_data })
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct ServiceDataAdvertisementEvent {
    pub service_data: HashMap<Uuid, String>,
}

impl ServiceDataAdvertisementEvent {
    pub(crate) fn new(
        local_name: Option<String>,
        rssi: Option<i16>,
        id: String,
        service_data: HashMap<Uuid, String>
    ) -> Event<Self> {
        Event::new("ServiceDataAdvertisementEvent".into(), local_name, rssi, id, ServiceDataAdvertisementEvent { service_data })
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct ServicesAdvertisementEvent {
    pub services: Vec<Uuid>,
}

impl ServicesAdvertisementEvent {
    pub(crate) fn new(
        local_name: Option<String>,
        rssi: Option<i16>,
        id: String,
        services: Vec<Uuid>
    ) -> Event<Self> {
        Event::new("ServicesAdvertisement".into(), local_name, rssi, id, ServicesAdvertisementEvent { services })
    }
}
