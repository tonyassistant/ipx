use ipx::app::App;
use ipx::network::sample_interfaces;

#[test]
fn selection_wraps_forward() {
    let interfaces = sample_interfaces();
    let mut app = App::new(interfaces.clone());
    app.selected = interfaces.len() - 1;
    app.next();
    assert_eq!(app.selected, 0);
}

#[test]
fn selection_wraps_backward() {
    let interfaces = sample_interfaces();
    let mut app = App::new(interfaces.clone());
    app.selected = 0;
    app.previous();
    assert_eq!(app.selected, interfaces.len() - 1);
}

#[test]
fn palette_help_adds_log_entry() {
    let mut app = App::new(sample_interfaces());
    app.open_palette();
    app.palette = "help".into();
    app.execute_palette();
    assert!(app
        .log
        .iter()
        .any(|entry| entry.contains("available commands")));
}
