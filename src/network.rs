use std::process::Command;

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

pub fn load_interfaces() -> Vec<NetworkInterface> {
    discover_macos_interfaces().unwrap_or_else(sample_interfaces)
}

fn discover_macos_interfaces() -> Option<Vec<NetworkInterface>> {
    if !cfg!(target_os = "macos") {
        return None;
    }

    let output = Command::new("networksetup")
        .arg("-listallhardwareports")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8(output.stdout).ok()?;
    let mut interfaces = parse_networksetup_hardwareports(&text);
    if interfaces.is_empty() {
        return None;
    }

    let ifconfig_output = Command::new("ifconfig").output().ok();
    let ifconfig_text = ifconfig_output
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .unwrap_or_default();

    for iface in &mut interfaces {
        enrich_from_ifconfig(iface, &ifconfig_text);
    }

    Some(interfaces)
}

pub fn parse_networksetup_hardwareports(input: &str) -> Vec<NetworkInterface> {
    let mut result = Vec::new();
    let mut name: Option<String> = None;
    let mut device: Option<String> = None;
    let mut mac: Option<String> = None;

    for line in input.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Hardware Port: ") {
            if let (Some(name), Some(device)) = (name.take(), device.take()) {
                result.push(NetworkInterface {
                    kind: classify_kind(&name, &device),
                    status: InterfaceStatus::Inactive,
                    ipv4: None,
                    mac: mac.take(),
                    notes: vec!["Imported from networksetup".to_string()],
                    name,
                    device,
                });
            }
            name = Some(rest.to_string());
            mac = None;
        } else if let Some(rest) = trimmed.strip_prefix("Device: ") {
            device = Some(rest.to_string());
        } else if let Some(rest) = trimmed.strip_prefix("Ethernet Address: ") {
            mac = Some(rest.to_string());
        }
    }

    if let (Some(name), Some(device)) = (name.take(), device.take()) {
        result.push(NetworkInterface {
            kind: classify_kind(&name, &device),
            status: InterfaceStatus::Inactive,
            ipv4: None,
            mac,
            notes: vec!["Imported from networksetup".to_string()],
            name,
            device,
        });
    }

    result
}

fn enrich_from_ifconfig(iface: &mut NetworkInterface, ifconfig_text: &str) {
    let mut in_block = false;
    for line in ifconfig_text.lines() {
        if !line.starts_with('\t') && !line.starts_with(' ') {
            in_block = line.starts_with(&format!("{}:", iface.device));
            continue;
        }

        if !in_block {
            continue;
        }

        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("inet ") {
            let ip = rest.split_whitespace().next().unwrap_or("");
            if !ip.is_empty() {
                iface.ipv4 = Some(ip.to_string());
            }
        }

        if trimmed.contains("status: active") {
            iface.status = InterfaceStatus::Connected;
        } else if trimmed.contains("status: inactive") {
            iface.status = InterfaceStatus::Inactive;
        }

        if iface.status == InterfaceStatus::Inactive && trimmed.contains("status:") {
            iface.status = InterfaceStatus::Disconnected;
        }
    }
}

fn classify_kind(name: &str, device: &str) -> InterfaceKind {
    let l = format!("{} {}", name.to_lowercase(), device.to_lowercase());
    if l.contains("wi-fi") || l.contains("airport") || l.contains("wireless") {
        InterfaceKind::Wireless
    } else if l.contains("bridge") {
        InterfaceKind::Bridge
    } else if l.contains("ethernet") {
        InterfaceKind::Ethernet
    } else if l.contains("utun") || l.contains("awdl") || l.contains("llw") || l.contains("virtual")
    {
        InterfaceKind::Virtual
    } else {
        InterfaceKind::Unknown
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
