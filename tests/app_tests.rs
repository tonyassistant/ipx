use ipx::app::{App, DetailTab, Focus};
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

#[test]
fn palette_unknown_command_is_logged() {
    let mut app = App::new(sample_interfaces());
    app.open_palette();
    app.palette = "kaboom".into();
    app.execute_palette();
    assert!(app
        .log
        .iter()
        .any(|entry| entry.contains("unknown command: kaboom")));
}

#[test]
fn tab_cycle_moves_through_all_views() {
    let mut app = App::new(sample_interfaces());
    assert_eq!(app.detail_tab, DetailTab::Overview);
    app.next_tab();
    assert_eq!(app.detail_tab, DetailTab::Signals);
    app.next_tab();
    assert_eq!(app.detail_tab, DetailTab::Actions);
    app.next_tab();
    assert_eq!(app.detail_tab, DetailTab::Overview);
}

#[test]
fn palette_open_close_changes_focus() {
    let mut app = App::new(sample_interfaces());
    app.open_palette();
    assert_eq!(app.focus, Focus::Palette);
    app.close_palette();
    assert_eq!(app.focus, Focus::List);
}

#[test]
fn interface_counts_match_sample_data() {
    let app = App::new(sample_interfaces());
    let counts = app.interface_counts();
    assert_eq!(counts.connected, 1);
    assert_eq!(counts.disconnected, 1);
    assert_eq!(counts.inactive, 1);
}

#[test]
fn selection_label_tracks_selected_row() {
    let mut app = App::new(sample_interfaces());
    assert_eq!(app.selection_label(), "1/3");
    app.selected = 2;
    assert_eq!(app.selection_label(), "3/3");
}

#[test]
fn palette_suggestions_include_operator_commands() {
    let app = App::new(sample_interfaces());
    assert!(app.palette_suggestions().contains(&"refresh"));
    assert!(app.palette_suggestions().contains(&"quit"));
}
