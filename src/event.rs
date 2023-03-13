use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Event {
    pub id: String,

    pub event: String,

    pub mac_address: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rssi: Option<i16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer_data: Option<HashMap<u16, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_data: Option<HashMap<Uuid, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub services: Option<Vec<Uuid>>,
}

impl Event {
    pub(crate) fn new(
        id: String,
        event: String,
        mac_address: String,
        local_name: Option<String>,
        rssi: Option<i16>,
        manufacturer_data: Option<HashMap<u16, String>>,
        service_data: Option<HashMap<Uuid, String>>,
        services: Option<Vec<Uuid>>,
    ) -> Self {
        Self {
            id,
            event,
            mac_address,
            local_name,
            rssi,
            manufacturer_data,
            service_data,
            services,
        }
    }
}
