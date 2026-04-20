use ipx::network::{
    build_interfaces_from_linux_outputs, build_interfaces_from_macos_outputs,
    build_interfaces_from_windows_ipconfig, parse_linux_ip_link, parse_networksetup_hardwareports,
    parse_networksetup_service_order, sample_interfaces, InterfaceKind, InterfaceStatus,
    NetworkServiceStatus, ReachabilityState,
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

    let routes = r#"
Internet:
default            192.168.1.1        UGScg                 en0
"#;

    let interfaces = build_interfaces_from_macos_outputs(hardware, service_order, ifconfig, routes);
    assert_eq!(interfaces.len(), 2);

    let wifi = interfaces
        .iter()
        .find(|iface| iface.device == "en0")
        .unwrap();
    assert_eq!(wifi.status, InterfaceStatus::Connected);
    assert_eq!(wifi.ipv4.as_deref(), Some("192.168.1.20"));
    assert_eq!(wifi.services.len(), 2);
    assert_eq!(wifi.gateway.as_deref(), Some("192.168.1.1"));
    assert_eq!(wifi.reachability(), ReachabilityState::PrivateRoute);
    assert_eq!(wifi.services[0].name, "Wi-Fi");
    assert_eq!(wifi.services[0].status, NetworkServiceStatus::Enabled);
    assert_eq!(wifi.services[1].name, "Wi-Fi Diagnostics");
    assert_eq!(wifi.services[1].status, NetworkServiceStatus::Disabled);
    assert!(wifi
        .notes
        .iter()
        .any(|note| note.contains("Primary service: Wi-Fi (enabled) • priority 1")));

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
fn globally_routed_ip_and_gateway_are_treated_as_reachable() {
    let mut interfaces = sample_interfaces();
    interfaces[0].ipv4 = Some("8.8.8.8".to_string());
    interfaces[0].gateway = Some("1.1.1.1".to_string());

    assert_eq!(interfaces[0].reachability(), ReachabilityState::Reachable);
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

#[test]
fn default_route_presence_marks_primary_uplink_role() {
    let interfaces = sample_interfaces();
    assert!(interfaces[0].carries_default_route());
    assert!(!interfaces[1].carries_default_route());
}

#[test]
fn parses_linux_ip_link_output() {
    let input = r#"
1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536 state UNKNOWN mode DEFAULT group default
2: eth0@if3: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 state UP mode DEFAULT group default link/ether 52:54:00:12:34:56
3: docker0: <NO-CARRIER,BROADCAST,MULTICAST,UP> mtu 1500 state DOWN mode DEFAULT group default link/ether 02:42:aa:bb:cc:dd
"#;

    let interfaces = parse_linux_ip_link(input);
    assert_eq!(interfaces.len(), 3);
    assert_eq!(interfaces[1].device, "eth0");
    assert_eq!(interfaces[1].kind, InterfaceKind::Ethernet);
    assert_eq!(interfaces[1].status, InterfaceStatus::Connected);
    assert_eq!(interfaces[2].kind, InterfaceKind::Virtual);
}

#[test]
fn builds_linux_interfaces_with_addresses_and_gateway() {
    let link = r#"
2: eth0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 state UP mode DEFAULT group default link/ether 52:54:00:12:34:56
3: docker0: <NO-CARRIER,BROADCAST,MULTICAST,UP> mtu 1500 state DOWN mode DEFAULT group default link/ether 02:42:aa:bb:cc:dd
"#;
    let addr = r#"
2: eth0    inet 192.168.1.50/24 brd 192.168.1.255 scope global dynamic eth0
3: docker0    inet 172.17.0.1/16 brd 172.17.255.255 scope global docker0
"#;
    let route = r#"
default via 192.168.1.1 dev eth0 proto dhcp src 192.168.1.50 metric 100
default via 10.0.0.1 dev wlan0 proto dhcp src 10.0.0.8 metric 600
"#;

    let interfaces = build_interfaces_from_linux_outputs(link, addr, route);
    let eth0 = interfaces
        .iter()
        .find(|iface| iface.device == "eth0")
        .unwrap();
    assert_eq!(eth0.ipv4.as_deref(), Some("192.168.1.50"));
    assert_eq!(eth0.gateway.as_deref(), Some("192.168.1.1"));
    let default_route = eth0.default_route.as_ref().unwrap();
    assert_eq!(default_route.gateway, "192.168.1.1");
    assert_eq!(default_route.metric, Some(100));
    assert_eq!(default_route.source.as_deref(), Some("192.168.1.50"));
    assert_eq!(eth0.reachability(), ReachabilityState::PrivateRoute);

    let docker0 = interfaces
        .iter()
        .find(|iface| iface.device == "docker0")
        .unwrap();
    assert_eq!(docker0.kind, InterfaceKind::Virtual);
}

#[test]
fn builds_windows_interfaces_from_ipconfig() {
    let input = r#"
Windows IP Configuration

Ethernet adapter Ethernet:
   Connection-specific DNS Suffix  . :
   Description . . . . . . . . . . . : Intel(R) Ethernet Connection
   Physical Address. . . . . . . . . : AA-BB-CC-DD-EE-FF
   DHCP Enabled. . . . . . . . . . . : Yes
   IPv4 Address. . . . . . . . . . . : 10.0.0.25(Preferred)
   Default Gateway . . . . . . . . . : 10.0.0.1

Ethernet adapter vEthernet (WSL):
   Connection-specific DNS Suffix  . :
   Description . . . . . . . . . . . : Hyper-V Virtual Ethernet Adapter
   Physical Address. . . . . . . . . : 11-22-33-44-55-66
   Media State . . . . . . . . . . . : Media disconnected
"#;

    let interfaces = build_interfaces_from_windows_ipconfig(input);
    assert_eq!(interfaces.len(), 2);
    assert_eq!(interfaces[0].kind, InterfaceKind::Ethernet);
    assert_eq!(interfaces[0].ipv4.as_deref(), Some("10.0.0.25"));
    assert_eq!(interfaces[0].gateway.as_deref(), Some("10.0.0.1"));
    assert_eq!(interfaces[1].kind, InterfaceKind::Virtual);
    assert_eq!(interfaces[1].status, InterfaceStatus::Disconnected);
}
