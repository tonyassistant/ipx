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
    pub gateway: Option<String>,
    pub services: Vec<NetworkService>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReachabilityState {
    Reachable,
    LocalOnly,
    Down,
    Unknown,
}

impl ReachabilityState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Reachable => "reachable",
            Self::LocalOnly => "local only",
            Self::Down => "down",
            Self::Unknown => "unknown",
        }
    }

    pub fn note(&self) -> &'static str {
        match self {
            Self::Reachable => "Internet path appears available",
            Self::LocalOnly => "Interface has a local address but no verified upstream path",
            Self::Down => "No active path detected for this interface",
            Self::Unknown => "Reachability has not been evaluated yet",
        }
    }
}

impl NetworkInterface {
    pub fn reachability(&self) -> ReachabilityState {
        match self.status {
            InterfaceStatus::Connected => {
                if self.has_verified_upstream_path() {
                    ReachabilityState::Reachable
                } else if self.ipv4.is_some() {
                    ReachabilityState::LocalOnly
                } else {
                    ReachabilityState::LocalOnly
                }
            }
            InterfaceStatus::Disconnected => ReachabilityState::Down,
            InterfaceStatus::Inactive => ReachabilityState::Unknown,
        }
    }

    fn has_verified_upstream_path(&self) -> bool {
        match (self.ipv4.as_deref(), self.gateway.as_deref()) {
            (Some(ip), Some(gateway)) => {
                !is_private_or_special_ipv4(gateway) && !is_private_or_special_ipv4(ip)
            }
            (Some(ip), None) => is_globally_routable_ipv4(ip),
            _ => false,
        }
    }

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
            format!("Reachability: {}", self.reachability().label()),
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
    discover_interfaces().unwrap_or_else(sample_interfaces)
}

fn discover_interfaces() -> Option<Vec<NetworkInterface>> {
    if cfg!(target_os = "macos") {
        discover_macos_interfaces()
    } else if cfg!(target_os = "linux") {
        discover_linux_interfaces()
    } else if cfg!(target_os = "windows") {
        discover_windows_interfaces()
    } else {
        None
    }
}

fn discover_macos_interfaces() -> Option<Vec<NetworkInterface>> {
    let hardware_text = run_command("networksetup", &["-listallhardwareports"])?;
    let service_order_text =
        run_command("networksetup", &["-listnetworkserviceorder"]).unwrap_or_default();
    let ifconfig_text = run_command("ifconfig", &[]).unwrap_or_default();
    let routes_text = run_command("netstat", &["-rn", "-f", "inet"]).unwrap_or_default();

    let interfaces = build_interfaces_from_macos_outputs(
        &hardware_text,
        &service_order_text,
        &ifconfig_text,
        &routes_text,
    );
    if interfaces.is_empty() {
        return None;
    }

    Some(interfaces)
}

fn discover_linux_interfaces() -> Option<Vec<NetworkInterface>> {
    let link_text = run_command("ip", &["-o", "link", "show"])?;
    let addr_text = run_command("ip", &["-o", "-4", "addr", "show"]).unwrap_or_default();
    let route_text = run_command("ip", &["route", "show", "default"]).unwrap_or_default();

    let interfaces = build_interfaces_from_linux_outputs(&link_text, &addr_text, &route_text);
    if interfaces.is_empty() {
        return None;
    }

    Some(interfaces)
}

fn discover_windows_interfaces() -> Option<Vec<NetworkInterface>> {
    let config_text = run_command("ipconfig", &["/all"])?;
    let interfaces = build_interfaces_from_windows_ipconfig(&config_text);
    if interfaces.is_empty() {
        return None;
    }

    Some(interfaces)
}

pub fn build_interfaces_from_macos_outputs(
    hardware_text: &str,
    service_order_text: &str,
    ifconfig_text: &str,
    routes_text: &str,
) -> Vec<NetworkInterface> {
    let mut interfaces = parse_networksetup_hardwareports(hardware_text);
    let services = parse_networksetup_service_order(service_order_text);
    attach_services(&mut interfaces, services);

    for iface in &mut interfaces {
        enrich_from_ifconfig(iface, ifconfig_text);
        iface.gateway = parse_default_gateway(routes_text, &iface.device);
        finalize_interface_notes(iface);
    }

    interfaces
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
                    gateway: None,
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
            gateway: None,
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
    if l.contains("wi-fi") || l.contains("wifi") || l.contains("airport") || l.contains("wireless") || l.contains("wlan") {
        InterfaceKind::Wireless
    } else if l.contains("bridge") {
        InterfaceKind::Bridge
    } else if is_likely_virtual(&l) {
        InterfaceKind::Virtual
    } else if is_likely_ethernet(&l) {
        InterfaceKind::Ethernet
    } else {
        InterfaceKind::Unknown
    }
}

fn is_likely_ethernet(value: &str) -> bool {
    value.contains("ethernet")
        || value.contains(" lan")
        || value.starts_with("lan ")
        || value.contains("thunderbolt")
        || value.contains("gigabit")
        || value.contains("10/100")
        || value.contains("10gbe")
        || value.contains("eth")
        || value.contains("enp")
}

fn is_likely_virtual(value: &str) -> bool {
    value.contains("utun")
        || value.contains("awdl")
        || value.contains("llw")
        || value.contains("virtual")
        || value.contains("docker")
        || value.contains("veth")
        || value.contains("vethernet")
        || value.contains("wsl")
        || value.contains("hyper-v")
        || value.contains("tun")
        || value.contains("tap")
        || value.contains("vmnet")
}

fn parse_default_gateway(routes_text: &str, device: &str) -> Option<String> {
    for line in routes_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("Routing tables")
            || trimmed.starts_with("Internet:")
        {
            continue;
        }

        let columns = trimmed.split_whitespace().collect::<Vec<_>>();
        if columns.len() < 4 {
            continue;
        }

        if columns[0] != "default" {
            continue;
        }

        if columns.last().copied() != Some(device) {
            continue;
        }

        return Some(columns[1].to_string());
    }

    None
}

pub fn build_interfaces_from_linux_outputs(
    link_text: &str,
    addr_text: &str,
    route_text: &str,
) -> Vec<NetworkInterface> {
    let mut interfaces = parse_linux_ip_link(link_text);

    for iface in &mut interfaces {
        enrich_from_linux_ip_addr(iface, addr_text);
        iface.gateway = parse_linux_default_gateway(route_text, &iface.device);
        finalize_interface_notes(iface);
    }

    interfaces
}

pub fn parse_linux_ip_link(input: &str) -> Vec<NetworkInterface> {
    let mut interfaces = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Some((_, remainder)) = trimmed.split_once(':') else {
            continue;
        };
        let remainder = remainder.trim();
        let Some((device_part, details_part)) = remainder.split_once(':') else {
            continue;
        };

        let device = device_part
            .trim()
            .split('@')
            .next()
            .unwrap_or_default()
            .to_string();
        let lower = details_part.to_lowercase();
        let status = if lower.contains("state up") {
            InterfaceStatus::Connected
        } else if lower.contains("state down") {
            InterfaceStatus::Disconnected
        } else {
            InterfaceStatus::Inactive
        };

        interfaces.push(NetworkInterface {
            name: device.clone(),
            device: device.clone(),
            kind: classify_kind(&device, &device),
            status,
            ipv4: None,
            mac: parse_linux_mac(details_part),
            gateway: None,
            services: Vec::new(),
            notes: vec!["Imported from ip link show".to_string()],
        });
    }

    interfaces
}

fn parse_linux_mac(details: &str) -> Option<String> {
    let columns = details.split_whitespace().collect::<Vec<_>>();
    columns
        .windows(2)
        .find(|window| window[0] == "link/ether")
        .map(|window| window[1].to_string())
}

fn enrich_from_linux_ip_addr(iface: &mut NetworkInterface, addr_text: &str) {
    for line in addr_text.lines() {
        let trimmed = line.trim();
        if !trimmed.contains(&format!(" {} ", iface.device))
            && !trimmed.ends_with(&format!(" {}", iface.device))
        {
            continue;
        }

        let columns = trimmed.split_whitespace().collect::<Vec<_>>();
        if let Some(inet_idx) = columns.iter().position(|column| *column == "inet") {
            if let Some(value) = columns.get(inet_idx + 1) {
                iface.ipv4 = value.split('/').next().map(str::to_string);
                if iface.status == InterfaceStatus::Disconnected {
                    iface.status = InterfaceStatus::Connected;
                }
            }
        }
    }
}

fn parse_linux_default_gateway(route_text: &str, device: &str) -> Option<String> {
    for line in route_text.lines() {
        let columns = line.split_whitespace().collect::<Vec<_>>();
        if columns.len() < 5 || columns.first().copied() != Some("default") {
            continue;
        }

        let via = columns
            .windows(2)
            .find(|window| window[0] == "via")?
            .get(1)?;
        let dev = columns
            .windows(2)
            .find(|window| window[0] == "dev")?
            .get(1)?;
        if *dev == device {
            return Some((*via).to_string());
        }
    }

    None
}

pub fn build_interfaces_from_windows_ipconfig(input: &str) -> Vec<NetworkInterface> {
    let mut interfaces = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current = empty_windows_interface();

    for line in input.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }

        if !line.starts_with(' ') && trimmed.ends_with(':') {
            if let Some(name) = current_name.take() {
                finalize_windows_interface(&mut current);
                current.name = name.clone();
                if current.device.is_empty() {
                    current.device = sanitize_windows_device_name(&name);
                }
                interfaces.push(current.clone());
                current = empty_windows_interface();
            }

            let name = trimmed.trim_end_matches(':').to_string();
            if name.contains("adapter") {
                current_name = Some(name.clone());
                current.name = name;
            }
            continue;
        }

        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let value = value.trim().to_string();
        let key = key.trim().to_lowercase();

        match key.as_str() {
            k if k.contains("physical address") => current.mac = Some(value),
            k if k.contains("ipv4 address") => {
                current.ipv4 = Some(value.trim_end_matches("(Preferred)").trim().to_string())
            }
            k if k.contains("default gateway") && !value.is_empty() => {
                current.gateway = Some(value)
            }
            k if k.contains("media state") => {
                current.status = if value.to_lowercase().contains("disconnected") {
                    InterfaceStatus::Disconnected
                } else {
                    InterfaceStatus::Connected
                }
            }
            k if k.contains("description") => current.notes.push(format!("Description: {value}")),
            _ => {}
        }
    }

    if let Some(name) = current_name {
        finalize_windows_interface(&mut current);
        current.name = name.clone();
        if current.device.is_empty() {
            current.device = sanitize_windows_device_name(&name);
        }
        interfaces.push(current);
    }

    interfaces
}

fn empty_windows_interface() -> NetworkInterface {
    NetworkInterface {
        name: String::new(),
        device: String::new(),
        kind: InterfaceKind::Unknown,
        status: InterfaceStatus::Inactive,
        ipv4: None,
        mac: None,
        gateway: None,
        services: Vec::new(),
        notes: vec!["Imported from ipconfig /all".to_string()],
    }
}

fn finalize_windows_interface(iface: &mut NetworkInterface) {
    iface.kind = classify_kind(&iface.name, &iface.device);
    if iface.ipv4.is_some() && iface.status == InterfaceStatus::Inactive {
        iface.status = InterfaceStatus::Connected;
    }
    finalize_interface_notes(iface);
}

fn sanitize_windows_device_name(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

fn is_globally_routable_ipv4(ip: &str) -> bool {
    !is_private_or_special_ipv4(ip)
}

fn is_private_or_special_ipv4(ip: &str) -> bool {
    let octets = ip
        .split('.')
        .map(str::parse::<u8>)
        .collect::<Result<Vec<_>, _>>();

    let Ok(octets) = octets else {
        return true;
    };

    if octets.len() != 4 {
        return true;
    }

    match octets.as_slice() {
        [10, ..] => true,
        [127, ..] => true,
        [169, 254, ..] => true,
        [172, second, ..] if (16..=31).contains(second) => true,
        [192, 168, ..] => true,
        [100, second, ..] if (64..=127).contains(second) => true,
        [0, ..] => true,
        [255, 255, 255, 255] => true,
        _ => false,
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
            gateway: Some("192.168.1.1".to_string()),
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
            gateway: None,
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
            gateway: None,
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
