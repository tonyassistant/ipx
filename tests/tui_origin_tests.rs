use ipx::network::{
    build_interfaces_from_linux_outputs, sample_interfaces,
};

fn render_overview_lines() -> String {
    let interfaces = sample_interfaces();
    let iface = &interfaces[0];
    let primary_service = iface
        .services
        .iter()
        .find(|service| service.status == ipx::network::NetworkServiceStatus::Enabled)
        .or_else(|| iface.services.first())
        .map(|service| service.name.clone())
        .unwrap_or_else(|| "-".to_string());
    let route_role = if iface.carries_default_route() {
        "primary uplink"
    } else {
        "secondary or local"
    };

    [
        format!("Source {}", iface.origin.label()),
        format!("Role {}", route_role),
        format!("Service {}", primary_service),
    ]
    .join("\n")
}

#[test]
fn sample_overview_reflects_origin_label() {
    let rendered = render_overview_lines();
    assert!(rendered.contains("Source Sample data"));
}

#[test]
fn linux_interfaces_expose_platform_service_mapping_note() {
    let interfaces = build_interfaces_from_linux_outputs(
        "2: eth0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 state UP mode DEFAULT group default link/ether 52:54:00:12:34:56",
        "2: eth0    inet 192.168.1.50/24 brd 192.168.1.255 scope global dynamic eth0",
        "default via 192.168.1.1 dev eth0 proto dhcp src 192.168.1.50 metric 100",
    );

    let eth0 = interfaces.iter().find(|iface| iface.device == "eth0").unwrap();
    assert!(eth0
        .origin
        .service_mapping_note()
        .contains("not yet available on Linux"));
}
