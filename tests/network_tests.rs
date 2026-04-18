use ipx::network::{
    parse_networksetup_hardwareports, parse_networksetup_service_order, sample_interfaces,
    InterfaceKind, NetworkServiceStatus,
};

#[test]
fn parses_networksetup_blocks() {
    let input = r#"
Hardware Port: Wi-Fi
Device: en0
Ethernet Address: aa:bb:cc:dd:ee:ff

Hardware Port: USB 10/100/1000 LAN
Device: en7
Ethernet Address: 11:22:33:44:55:66
"#;

    let interfaces = parse_networksetup_hardwareports(input);
    assert_eq!(interfaces.len(), 2);
    assert_eq!(interfaces[0].name, "Wi-Fi");
    assert_eq!(interfaces[0].device, "en0");
    assert_eq!(interfaces[0].kind, InterfaceKind::Wireless);
    assert_eq!(interfaces[0].services.len(), 0);
    assert_eq!(interfaces[1].device, "en7");
}

#[test]
fn parses_networksetup_service_order_with_device_mapping() {
    let input = r#"
An asterisk (*) denotes that a network service is disabled.
(1) Wi-Fi
(Hardware Port: Wi-Fi, Device: en0)
(2) USB 10/100/1000 LAN
(Hardware Port: USB 10/100/1000 LAN, Device: en7)
(3) (*) Thunderbolt Bridge
(Hardware Port: Thunderbolt Bridge, Device: bridge0)
"#;

    let services = parse_networksetup_service_order(input);
    assert_eq!(services.len(), 3);
    assert_eq!(services[0].name, "Wi-Fi");
    assert_eq!(services[0].status, NetworkServiceStatus::Enabled);
    assert_eq!(services[0].order, Some(1));
    assert_eq!(services[0].device.as_deref(), Some("en0"));
    assert_eq!(services[2].status, NetworkServiceStatus::Disabled);
    assert_eq!(services[2].device.as_deref(), Some("bridge0"));
}

#[test]
fn sample_interfaces_include_service_model() {
    let interfaces = sample_interfaces();
    assert!(!interfaces[0].services.is_empty());
    assert_eq!(interfaces[0].services[0].name, "Wi-Fi");
    assert!(interfaces[0]
        .detail_lines()
        .iter()
        .any(|line| line.contains("Services: 1")));
}
