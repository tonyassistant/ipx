use ipx::network::{parse_networksetup_hardwareports, InterfaceKind};

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
    assert_eq!(interfaces[1].device, "en7");
}
