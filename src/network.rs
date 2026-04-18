use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub device: String,
    pub kind: String,
    pub status: String,
    pub ipv4: Option<String>,
    pub mac: Option<String>,
}

pub fn sample_interfaces() -> Vec<NetworkInterface> {
    vec![
        NetworkInterface {
            name: "Wi-Fi".to_string(),
            device: "en0".to_string(),
            kind: "wireless".to_string(),
            status: "connected".to_string(),
            ipv4: Some("192.168.1.24".to_string()),
            mac: Some("ac:de:48:00:11:22".to_string()),
        },
        NetworkInterface {
            name: "Thunderbolt Bridge".to_string(),
            device: "bridge0".to_string(),
            kind: "bridge".to_string(),
            status: "inactive".to_string(),
            ipv4: None,
            mac: Some("ac:de:48:00:11:33".to_string()),
        },
        NetworkInterface {
            name: "USB Ethernet".to_string(),
            device: "en7".to_string(),
            kind: "ethernet".to_string(),
            status: "disconnected".to_string(),
            ipv4: None,
            mac: Some("ac:de:48:00:11:44".to_string()),
        },
    ]
}
