use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceKind {
    Wireless,
    Ethernet,
    Bridge,
    Virtual,
    Unknown,
}

impl InterfaceKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Wireless => "wireless",
            Self::Ethernet => "ethernet",
            Self::Bridge => "bridge",
            Self::Virtual => "virtual",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceStatus {
    Connected,
    Disconnected,
    Inactive,
}

impl InterfaceStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::Disconnected => "disconnected",
            Self::Inactive => "inactive",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub device: String,
    pub kind: InterfaceKind,
    pub status: InterfaceStatus,
    pub ipv4: Option<String>,
    pub mac: Option<String>,
    pub notes: Vec<String>,
}

impl NetworkInterface {
    pub fn summary(&self) -> String {
        format!("{} ({}) [{}]", self.name, self.device, self.status.label())
    }

    pub fn detail_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("Name: {}", self.name),
            format!("Device: {}", self.device),
            format!("Kind: {}", self.kind.label()),
            format!("Status: {}", self.status.label()),
            format!("IPv4: {}", self.ipv4.as_deref().unwrap_or("-")),
            format!("MAC: {}", self.mac.as_deref().unwrap_or("-")),
        ];

        if !self.notes.is_empty() {
            lines.push(String::new());
            lines.push("Notes:".to_string());
            lines.extend(self.notes.iter().map(|note| format!("- {note}")));
        }

        lines
    }
}

pub fn sample_interfaces() -> Vec<NetworkInterface> {
    vec![
        NetworkInterface {
            name: "Wi-Fi".to_string(),
            device: "en0".to_string(),
            kind: InterfaceKind::Wireless,
            status: InterfaceStatus::Connected,
            ipv4: Some("192.168.1.24".to_string()),
            mac: Some("ac:de:48:00:11:22".to_string()),
            notes: vec![
                "Primary uplink".to_string(),
                "RSSI visibility planned for next parser pass".to_string(),
            ],
        },
        NetworkInterface {
            name: "USB Ethernet".to_string(),
            device: "en7".to_string(),
            kind: InterfaceKind::Ethernet,
            status: InterfaceStatus::Disconnected,
            ipv4: None,
            mac: Some("ac:de:48:00:11:44".to_string()),
            notes: vec!["Cable not detected".to_string()],
        },
        NetworkInterface {
            name: "Thunderbolt Bridge".to_string(),
            device: "bridge0".to_string(),
            kind: InterfaceKind::Bridge,
            status: InterfaceStatus::Inactive,
            ipv4: None,
            mac: Some("ac:de:48:00:11:33".to_string()),
            notes: vec!["Available for peer networking".to_string()],
        },
    ]
}
