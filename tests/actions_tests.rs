use ipx::{
    actions::{action_catalog, execute_action, ActionEffect, ActionKind, ActionSafety},
    app::{App, DetailTab},
    network::sample_interfaces,
};

#[test]
fn action_catalog_marks_read_only_and_confirmed_actions() {
    let iface = &sample_interfaces()[0];
    let actions = action_catalog(iface);

    assert_eq!(actions[0].kind, ActionKind::RefreshState);
    assert_eq!(actions[0].safety, ActionSafety::ReadOnly);
    assert_eq!(actions[3].kind, ActionKind::RenewDhcpLease);
    assert_eq!(actions[3].safety, ActionSafety::ConfirmRequired);
    assert!(!actions[3].enabled);
}

#[test]
fn inspect_action_focuses_overview() {
    let iface = &sample_interfaces()[0];
    let spec = action_catalog(iface)
        .into_iter()
        .find(|spec| spec.kind == ActionKind::InspectServices)
        .unwrap();

    let execution = execute_action(&spec, iface);
    assert_eq!(execution.effect, ActionEffect::FocusOverview);
}

#[test]
fn mutating_action_requires_confirmation_and_stays_blocked() {
    let mut app = App::new(sample_interfaces());
    app.invoke_action(ActionKind::RenewDhcpLease);
    assert!(app.pending_confirmation.is_some());

    app.confirm_pending_action();
    assert!(app.pending_confirmation.is_none());
    assert_eq!(app.status_line, "Mutating actions stay disabled in v1");
}

#[test]
fn selected_action_execution_updates_detail_focus() {
    let mut app = App::new(sample_interfaces());
    app.detail_tab = DetailTab::Actions;
    app.action_selected = 2;

    app.invoke_selected_action();

    assert_eq!(app.detail_tab, DetailTab::Overview);
    assert!(app
        .log
        .iter()
        .any(|entry| entry.contains("inspecting services")));
}
