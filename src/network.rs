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
pub enum NetworkServiceStatus {
    Enabled,
    Disabled,
    Unknown,
}

impl NetworkServiceStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkService {
    pub name: String,
    pub status: NetworkServiceStatus,
    pub order: Option<usize>,
    pub port: Option<String>,
    pub device: Option<String>,
}

impl NetworkService {
    pub fn summary(&self) -> String {
        let mut parts = vec![self.name.clone(), self.status.label().to_string()];

        if let Some(order) = self.order {
            parts.push(format!("priority {order}"));
        }

        if let Some(port) = &self.port {
            parts.push(port.clone());
        }

        parts.join(" • ")
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
    pub services: Vec<NetworkService>,
    pub notes: Vec<String>,
}

impl NetworkInterface {
    #[allow(dead_code)]
    pub fn summary(&self) -> String {
        format!("{} ({}) [{}]", self.name, self.device, self.status.label())
    }

    #[allow(dead_code)]
    pub fn detail_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("Name: {}", self.name),
            format!("Device: {}", self.device),
            format!("Kind: {}", self.kind.label()),
            format!("Status: {}", self.status.label()),
            format!("IPv4: {}", self.ipv4.as_deref().unwrap_or("-")),
            format!("MAC: {}", self.mac.as_deref().unwrap_or("-")),
        ];

        if self.services.is_empty() {
            lines.push("Services: -".to_string());
        } else {
            lines.push(format!("Services: {}", self.services.len()));
            lines.extend(
                self.services
                    .iter()
                    .map(|service| format!("  • {}", service.summary())),
            );
        }

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

    let hardware_text = run_command("networksetup", &["-listallhardwareports"])?;
    let mut interfaces = parse_networksetup_hardwareports(&hardware_text);
    if interfaces.is_empty() {
        return None;
    }

    let service_order_text =
        run_command("networksetup", &["-listnetworkserviceorder"]).unwrap_or_default();
    let services = parse_networksetup_service_order(&service_order_text);
    attach_services(&mut interfaces, services);

    let ifconfig_text = run_command("ifconfig", &[]).unwrap_or_default();
    for iface in &mut interfaces {
        enrich_from_ifconfig(iface, &ifconfig_text);
        finalize_interface_notes(iface);
    }

    Some(interfaces)
}

fn run_command(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout).ok()
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
                    services: Vec::new(),
                    notes: vec!["Imported from networksetup hardware ports".to_string()],
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
            services: Vec::new(),
            notes: vec!["Imported from networksetup hardware ports".to_string()],
            name,
            device,
        });
    }

    result
}

pub fn parse_networksetup_service_order(input: &str) -> Vec<NetworkService> {
    let mut services = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_status = NetworkServiceStatus::Unknown;
    let mut current_order: Option<usize> = None;

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some((order, name, status)) = parse_service_header(line) {
            if let Some(existing_name) = current_name.take() {
                services.push(NetworkService {
                    name: existing_name,
                    status: current_status,
                    order: current_order,
                    port: None,
                    device: None,
                });
            }

            current_name = Some(name);
            current_status = status;
            current_order = Some(order);
            continue;
        }

        if let Some(name) = line.strip_prefix("(*) ") {
            services.push(NetworkService {
                name: name.to_string(),
                status: NetworkServiceStatus::Disabled,
                order: None,
                port: None,
                device: None,
            });
            continue;
        }

        if let Some(name) =
            line.strip_prefix("An asterisk (*) denotes that a network service is disabled.")
        {
            let _ = name;
            continue;
        }

        if let Some(rest) = line.strip_prefix("(Hardware Port: ") {
            if let Some(service) = services.last_mut().filter(|_| current_name.is_none()) {
                let (port, device) = parse_service_mapping(rest);
                service.port = port;
                service.device = device;
            } else if let Some(service_name) = current_name.take() {
                let (port, device) = parse_service_mapping(rest);
                services.push(NetworkService {
                    name: service_name,
                    status: current_status,
                    order: current_order,
                    port,
                    device,
                });
                current_status = NetworkServiceStatus::Unknown;
                current_order = None;
            }
        }
    }

    if let Some(name) = current_name {
        services.push(NetworkService {
            name,
            status: current_status,
            order: current_order,
            port: None,
            device: None,
        });
    }

    services
}

fn parse_service_header(line: &str) -> Option<(usize, String, NetworkServiceStatus)> {
    let first_paren_end = line.find(')')?;
    let order = line[1..first_paren_end].parse().ok()?;
    let remainder = line.get(first_paren_end + 1..)?.trim();
    let (name, status) = if let Some(name) = remainder.strip_prefix("(*) ") {
        (name.trim().to_string(), NetworkServiceStatus::Disabled)
    } else {
        (remainder.to_string(), NetworkServiceStatus::Enabled)
    };

    Some((order, name, status))
}

fn parse_service_mapping(input: &str) -> (Option<String>, Option<String>) {
    let trimmed = input.trim_end_matches(')');
    let mut port = None;
    let mut device = None;

    for part in trimmed.split(", ") {
        if let Some(value) = part.strip_prefix("Hardware Port: ") {
            port = Some(value.to_string());
        } else if let Some(value) = part.strip_prefix("Device: ") {
            device = Some(value.to_string());
        }
    }

    (port, device)
}

fn attach_services(interfaces: &mut [NetworkInterface], services: Vec<NetworkService>) {
    for service in services {
        let Some(device) = service.device.as_deref() else {
            continue;
        };

        if let Some(interface) = interfaces.iter_mut().find(|iface| iface.device == device) {
            interface.services.push(service);
        }
    }

    for interface in interfaces {
        interface
            .services
            .sort_by_key(|service| service.order.unwrap_or(usize::MAX));
    }
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

        if let Some(rest) = trimmed.strip_prefix("ether ") {
            let mac = rest.split_whitespace().next().unwrap_or("");
            if !mac.is_empty() && iface.mac.is_none() {
                iface.mac = Some(mac.to_string());
            }
        }

        if trimmed.contains("status: active") {
            iface.status = InterfaceStatus::Connected;
        } else if trimmed.contains("status: inactive") {
            iface.status = InterfaceStatus::Inactive;
        } else if iface.status == InterfaceStatus::Inactive && trimmed.contains("status:") {
            iface.status = InterfaceStatus::Disconnected;
        }
    }

    if iface.ipv4.is_some() && iface.status == InterfaceStatus::Inactive {
        iface.status = InterfaceStatus::Connected;
    }
}

fn finalize_interface_notes(iface: &mut NetworkInterface) {
    if iface.services.is_empty() {
        iface
            .notes
            .push("No mapped network services discovered for this device".to_string());
    } else {
        let primary = iface
            .services
            .iter()
            .find(|service| service.status == NetworkServiceStatus::Enabled)
            .or_else(|| iface.services.first());

        if let Some(service) = primary {
            iface.notes.push(format!(
                "Primary service: {} ({})",
                service.name,
                service.status.label()
            ));
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
            services: vec![NetworkService {
                name: "Wi-Fi".to_string(),
                status: NetworkServiceStatus::Enabled,
                order: Some(1),
                port: Some("Wi-Fi".to_string()),
                device: Some("en0".to_string()),
            }],
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
            services: vec![NetworkService {
                name: "USB 10/100/1000 LAN".to_string(),
                status: NetworkServiceStatus::Enabled,
                order: Some(2),
                port: Some("USB 10/100/1000 LAN".to_string()),
                device: Some("en7".to_string()),
            }],
            notes: vec!["Cable not detected".to_string()],
        },
        NetworkInterface {
            name: "Thunderbolt Bridge".to_string(),
            device: "bridge0".to_string(),
            kind: InterfaceKind::Bridge,
            status: InterfaceStatus::Inactive,
            ipv4: None,
            mac: Some("ac:de:48:00:11:33".to_string()),
            services: vec![NetworkService {
                name: "Thunderbolt Bridge".to_string(),
                status: NetworkServiceStatus::Enabled,
                order: Some(3),
                port: Some("Thunderbolt Bridge".to_string()),
                device: Some("bridge0".to_string()),
            }],
            notes: vec!["Available for peer networking".to_string()],
        },
    ]
}
