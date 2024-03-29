use envconfig::Envconfig;

#[derive(Envconfig)]
pub struct Config {
    #[envconfig(from = "BLE_TO_MQTT_HOST")]
    pub mqtt_host: String,

    #[envconfig(from = "BLE_TO_MQTT_PORT", default = "1883")]
    pub mqtt_port: u16,

    #[envconfig(from = "BLE_TO_MQTT_USE_TLS_TRANSPORT", default = "false")]
    pub mqtt_use_tls_transport: bool,

    #[envconfig(from = "BLE_TO_MQTT_USERNAME")]
    pub mqtt_username: Option<String>,

    #[envconfig(from = "BLE_TO_MQTT_PASSWORD")]
    pub mqtt_password: Option<String>,

    #[envconfig(from = "BLE_TO_MQTT_CLIENT_ID", default = "ble-to-mqtt-bridge")]
    pub mqtt_client_id: String,

    #[envconfig(from = "BLE_TO_MQTT_TOPIC")]
    pub mqtt_topic: String,

    #[envconfig(from = "BLE_TO_MQTT_TOPIC_QOS", default = "0")]
    pub mqtt_topic_qos: u8,

    #[envconfig(from = "BLE_TO_MQTT_KEEP_ALIVE_INTERVAL_SEC", default = "10")]
    pub mqtt_keep_alive_interval_seconds: u64,

    #[envconfig(from = "BLE_TO_MQTT_CLEAN_START", default = "false")]
    pub mqtt_clean_start: bool,

    #[envconfig(from = "BLE_TO_MQTT_BT_AUTO_SCAN_RESTART_INTERVAL_SEC", default = "3600")]
    pub bt_auto_scan_restart_interval_seconds: u64,
}
