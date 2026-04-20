use ipx::app::{App, DetailTab, Focus, InterfaceVisibility};
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
    app.dismiss_palette();
    assert_eq!(app.focus, Focus::List);
}

#[test]
fn executing_palette_command_preserves_resulting_status() {
    let mut app = App::new(sample_interfaces());
    app.open_palette();
    app.palette = "copy".into();
    app.execute_palette();

    assert_eq!(app.focus, Focus::List);
    assert_eq!(app.status_line, "Copied en0 summary");
    assert_eq!(
        app.action_feedback
            .as_ref()
            .map(|feedback| feedback.headline.as_str()),
        Some("Summary copied")
    );
}

#[test]
fn dismissing_palette_without_command_resets_status() {
    let mut app = App::new(sample_interfaces());
    app.open_palette();
    app.dismiss_palette();

    assert_eq!(app.status_line, "Ready");
    assert!(app.action_feedback.is_none());
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
    assert!(app.palette_suggestions().contains(&"show active"));
    assert!(app.palette_suggestions().contains(&"renew"));
    assert!(app.palette_suggestions().contains(&"quit"));
}

#[test]
fn can_filter_to_active_interfaces_only() {
    let mut app = App::new(sample_interfaces());
    app.set_interface_visibility(InterfaceVisibility::ActiveOnly);

    let visible = app.visible_interfaces();
    assert_eq!(visible.len(), 2);
    assert!(visible
        .iter()
        .all(|(_, iface)| iface.status != ipx::network::InterfaceStatus::Inactive));
    assert_eq!(app.selection_label(), "1/2");
}

#[test]
fn can_filter_to_inactive_interfaces_only() {
    let mut app = App::new(sample_interfaces());
    app.set_interface_visibility(InterfaceVisibility::InactiveOnly);

    let visible = app.visible_interfaces();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].1.name, "Thunderbolt Bridge");
    assert_eq!(app.selection_label(), "1/1");
}

#[test]
fn cycling_visibility_rotates_through_modes() {
    let mut app = App::new(sample_interfaces());
    assert_eq!(app.interface_visibility, InterfaceVisibility::All);

    app.cycle_interface_visibility();
    assert_eq!(app.interface_visibility, InterfaceVisibility::ActiveOnly);

    app.cycle_interface_visibility();
    assert_eq!(app.interface_visibility, InterfaceVisibility::InactiveOnly);

    app.cycle_interface_visibility();
    assert_eq!(app.interface_visibility, InterfaceVisibility::All);
}

#[test]
fn filtered_palette_suggestions_prefer_prefix_matches() {
    let mut app = App::new(sample_interfaces());
    app.update_palette_input("re".into());

    assert_eq!(
        app.filtered_palette_suggestions(),
        vec!["renew", "reload", "refresh"]
    );
}

#[test]
fn palette_can_switch_visibility_modes() {
    let mut app = App::new(sample_interfaces());
    app.open_palette();
    app.palette = "show inactive".into();
    app.execute_palette();

    assert_eq!(app.interface_visibility, InterfaceVisibility::InactiveOnly);
    assert_eq!(app.selection_label(), "1/1");
    assert!(app
        .log
        .iter()
        .any(|entry| entry.contains("interface visibility set to inactive")));
}

#[test]
fn palette_selection_wraps_through_filtered_results() {
    let mut app = App::new(sample_interfaces());
    app.update_palette_input("re".into());

    app.select_next_palette_suggestion();
    assert_eq!(app.palette_selected, 1);

    app.select_next_palette_suggestion();
    assert_eq!(app.palette_selected, 2);

    app.select_next_palette_suggestion();
    assert_eq!(app.palette_selected, 0);

    app.select_previous_palette_suggestion();
    assert_eq!(app.palette_selected, 2);
}

#[test]
fn apply_selected_palette_suggestion_sets_palette_text() {
    let mut app = App::new(sample_interfaces());
    app.update_palette_input("re".into());
    app.select_next_palette_suggestion();

    assert!(app.apply_selected_palette_suggestion());
    assert_eq!(app.palette, "reload");
}

#[test]
fn applying_selected_palette_suggestion_without_query_uses_default_command() {
    let mut app = App::new(sample_interfaces());

    assert!(app.apply_selected_palette_suggestion());
    assert_eq!(app.palette, "copy");
}

#[test]
fn executing_empty_palette_without_query_keeps_palette_status() {
    let mut app = App::new(sample_interfaces());
    app.open_palette();
    app.select_next_palette_suggestion();
    app.select_next_palette_suggestion();
    app.execute_palette();

    assert_eq!(app.focus, Focus::List);
    assert_eq!(app.status_line, "Command palette");
}

#[test]
fn risky_action_sets_confirmation_feedback() {
    let mut app = App::new(sample_interfaces());
    app.detail_tab = DetailTab::Actions;
    app.action_selected = 3;

    app.invoke_selected_action();

    assert_eq!(
        app.action_feedback
            .as_ref()
            .map(|feedback| feedback.headline.as_str()),
        Some("Confirmation required")
    );
}

#[test]
fn cancelling_confirmation_sets_feedback() {
    let mut app = App::new(sample_interfaces());
    app.detail_tab = DetailTab::Actions;
    app.action_selected = 3;
    app.invoke_selected_action();

    app.cancel_pending_action();

    assert_eq!(
        app.action_feedback
            .as_ref()
            .map(|feedback| feedback.headline.as_str()),
        Some("Action cancelled")
    );
}
