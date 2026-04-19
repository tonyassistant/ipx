use ipx::network::{
    build_interfaces_from_macos_outputs, parse_networksetup_hardwareports,
    parse_networksetup_service_order, sample_interfaces, InterfaceKind, InterfaceStatus,
    NetworkServiceStatus,
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
    assert_eq!(interfaces[1].kind, InterfaceKind::Ethernet);
}

#[test]
fn classifies_realistic_macos_wired_adapter_names_as_ethernet() {
    let input = r#"
Hardware Port: Thunderbolt 10/100/1000 LAN
Device: en5
Ethernet Address: 22:33:44:55:66:77

Hardware Port: Belkin USB-C LAN
Device: en8
Ethernet Address: 33:44:55:66:77:88
"#;

    let interfaces = parse_networksetup_hardwareports(input);
    assert_eq!(interfaces.len(), 2);
    assert!(interfaces
        .iter()
        .all(|interface| interface.kind == InterfaceKind::Ethernet));
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
fn builds_interfaces_from_realistic_macos_outputs_with_primary_service_selection() {
    let hardware = r#"
Hardware Port: Wi-Fi
Device: en0
Ethernet Address: aa:bb:cc:dd:ee:ff

Hardware Port: iPhone USB
Device: en3
Ethernet Address: 66:55:44:33:22:11
"#;

    let service_order = r#"
An asterisk (*) denotes that a network service is disabled.
(1) Wi-Fi
(Hardware Port: Wi-Fi, Device: en0)
(*) Wi-Fi Diagnostics
(Hardware Port: Wi-Fi, Device: en0)
(*) iPhone USB
(Hardware Port: iPhone USB, Device: en3)
"#;

    let ifconfig = r#"
en0: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
    options=400<CHANNEL_IO>
    ether aa:bb:cc:dd:ee:ff
    inet 192.168.1.20 netmask 0xffffff00 broadcast 192.168.1.255
    status: active
en3: flags=8822<BROADCAST,SMART,SIMPLEX,MULTICAST> mtu 1500
    ether 66:55:44:33:22:11
    status: inactive
"#;

    let interfaces = build_interfaces_from_macos_outputs(hardware, service_order, ifconfig);
    assert_eq!(interfaces.len(), 2);

    let wifi = interfaces
        .iter()
        .find(|iface| iface.device == "en0")
        .unwrap();
    assert_eq!(wifi.status, InterfaceStatus::Connected);
    assert_eq!(wifi.ipv4.as_deref(), Some("192.168.1.20"));
    assert_eq!(wifi.services.len(), 2);
    assert_eq!(wifi.services[0].name, "Wi-Fi");
    assert_eq!(wifi.services[0].status, NetworkServiceStatus::Enabled);
    assert_eq!(wifi.services[1].name, "Wi-Fi Diagnostics");
    assert_eq!(wifi.services[1].status, NetworkServiceStatus::Disabled);
    assert!(wifi
        .notes
        .iter()
        .any(|note| note.contains("Primary service: Wi-Fi (enabled)")));

    let iphone_usb = interfaces
        .iter()
        .find(|iface| iface.device == "en3")
        .unwrap();
    assert_eq!(iphone_usb.status, InterfaceStatus::Inactive);
    assert_eq!(iphone_usb.services.len(), 1);
    assert_eq!(
        iphone_usb.services[0].status,
        NetworkServiceStatus::Disabled
    );
    assert!(iphone_usb
        .notes
        .iter()
        .any(|note| note.contains("Primary service: iPhone USB (disabled)")));
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
