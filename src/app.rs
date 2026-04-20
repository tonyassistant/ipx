use crate::{
    actions::{
        action_catalog, execute_action, pending_confirmation, ActionEffect, ActionKind,
        ActionOutcome, ActionSpec, PendingConfirmation,
    },
    network::{InterfaceStatus, NetworkInterface},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Palette,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Overview,
    Signals,
    Actions,
}

impl DetailTab {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Signals => "Signals",
            Self::Actions => "Actions",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Overview => Self::Signals,
            Self::Signals => Self::Actions,
            Self::Actions => Self::Overview,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Overview => Self::Actions,
            Self::Signals => Self::Overview,
            Self::Actions => Self::Signals,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterfaceCounts {
    pub connected: usize,
    pub disconnected: usize,
    pub inactive: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceVisibility {
    All,
    GroupInactive,
    ActiveOnly,
    InactiveOnly,
}

impl InterfaceVisibility {
    pub fn title(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::GroupInactive => "grouped",
            Self::ActiveOnly => "active",
            Self::InactiveOnly => "inactive",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::All => Self::GroupInactive,
            Self::GroupInactive => Self::ActiveOnly,
            Self::ActiveOnly => Self::InactiveOnly,
            Self::InactiveOnly => Self::All,
        }
    }

    pub fn includes(&self, status: &InterfaceStatus) -> bool {
        match self {
            Self::All | Self::GroupInactive => true,
            Self::ActiveOnly => *status != InterfaceStatus::Inactive,
            Self::InactiveOnly => *status == InterfaceStatus::Inactive,
        }
    }
}

#[derive(Debug)]
pub struct App {
    pub interfaces: Vec<NetworkInterface>,
    pub selected: usize,
    pub focus: Focus,
    pub detail_tab: DetailTab,
    pub palette: String,
    pub palette_history: Vec<String>,
    pub log: Vec<String>,
    pub should_quit: bool,
    pub status_line: String,
    pub action_feedback: Option<ActionOutcome>,
    pub action_selected: usize,
    pub pending_confirmation: Option<PendingConfirmation>,
    pub palette_selected: usize,
    pub interface_visibility: InterfaceVisibility,
}

impl App {
    pub fn new(interfaces: Vec<NetworkInterface>) -> Self {
        Self {
            interfaces,
            selected: 0,
            focus: Focus::List,
            detail_tab: DetailTab::Overview,
            palette: String::new(),
            palette_history: Vec::new(),
            log: vec!["ipx initialized".to_string()],
            should_quit: false,
            status_line: "Ready".to_string(),
            action_feedback: None,
            action_selected: 0,
            pending_confirmation: None,
            palette_selected: 0,
            interface_visibility: InterfaceVisibility::All,
        }
    }

    pub fn next(&mut self) {
        let visible = self.visible_interface_indexes();
        if visible.is_empty() {
            return;
        }

        let current_position = visible
            .iter()
            .position(|&idx| idx == self.selected)
            .unwrap_or(0);
        let next_position = (current_position + 1) % visible.len();
        self.selected = visible[next_position];
        self.action_selected = 0;
        self.status_line = format!(
            "Selected {}",
            self.selected_interface()
                .map(|i| i.name.clone())
                .unwrap_or_default()
        );
    }

    pub fn previous(&mut self) {
        let visible = self.visible_interface_indexes();
        if visible.is_empty() {
            return;
        }

        let current_position = visible
            .iter()
            .position(|&idx| idx == self.selected)
            .unwrap_or(0);
        let previous_position = if current_position == 0 {
            visible.len() - 1
        } else {
            current_position - 1
        };
        self.selected = visible[previous_position];
        self.action_selected = 0;
        self.status_line = format!(
            "Selected {}",
            self.selected_interface()
                .map(|i| i.name.clone())
                .unwrap_or_default()
        );
    }

    pub fn visible_interface_indexes(&self) -> Vec<usize> {
        self.interfaces
            .iter()
            .enumerate()
            .filter(|(_, iface)| self.interface_visibility.includes(&iface.status))
            .map(|(idx, _)| idx)
            .collect()
    }

    pub fn visible_interfaces(&self) -> Vec<(usize, &NetworkInterface)> {
        self.visible_interface_indexes()
            .into_iter()
            .filter_map(|idx| self.interfaces.get(idx).map(|iface| (idx, iface)))
            .collect()
    }

    pub fn grouped_interfaces(
        &self,
    ) -> (
        Vec<(usize, &NetworkInterface)>,
        Vec<(usize, &NetworkInterface)>,
    ) {
        let visible = self.visible_interfaces();
        if self.interface_visibility != InterfaceVisibility::GroupInactive {
            return (visible, Vec::new());
        }

        visible
            .into_iter()
            .partition(|(_, iface)| iface.status != InterfaceStatus::Inactive)
    }

    pub fn selected_interface(&self) -> Option<&NetworkInterface> {
        let visible = self.visible_interface_indexes();
        if visible.is_empty() {
            return None;
        }

        if visible.contains(&self.selected) {
            self.interfaces.get(self.selected)
        } else {
            self.interfaces.get(*visible.first().unwrap_or(&0))
        }
    }

    pub fn selection_label(&self) -> String {
        let visible = self.visible_interface_indexes();
        if visible.is_empty() {
            "No visible interfaces".to_string()
        } else {
            let current = visible
                .iter()
                .position(|&idx| idx == self.selected)
                .map(|idx| idx + 1)
                .unwrap_or(1);
            format!("{current}/{}", visible.len())
        }
    }

    pub fn interface_counts(&self) -> InterfaceCounts {
        self.interfaces.iter().fold(
            InterfaceCounts {
                connected: 0,
                disconnected: 0,
                inactive: 0,
            },
            |mut counts, iface| {
                match iface.status {
                    InterfaceStatus::Connected => counts.connected += 1,
                    InterfaceStatus::Disconnected => counts.disconnected += 1,
                    InterfaceStatus::Inactive => counts.inactive += 1,
                }
                counts
            },
        )
    }

    pub fn palette_suggestions(&self) -> &'static [&'static str] {
        &[
            "refresh",
            "reload",
            "show all",
            "group inactive",
            "show active",
            "show inactive",
            "help",
            "copy",
            "inspect",
            "renew",
            "quit",
        ]
    }

    pub fn filtered_palette_suggestions(&self) -> Vec<&'static str> {
        let query = self.palette.trim().to_lowercase();
        let mut ranked = self
            .palette_suggestions()
            .iter()
            .copied()
            .map(|command| (command, palette_match_rank(command, &query)))
            .filter(|(_, rank)| rank.is_some())
            .map(|(command, rank)| (command, rank.unwrap_or(u8::MAX)))
            .collect::<Vec<_>>();

        ranked.sort_by(|(left_command, left_rank), (right_command, right_rank)| {
            left_rank
                .cmp(right_rank)
                .then_with(|| left_command.len().cmp(&right_command.len()))
                .then_with(|| left_command.cmp(right_command))
        });

        ranked.into_iter().map(|(command, _)| command).collect()
    }

    pub fn open_palette(&mut self) {
        self.focus = Focus::Palette;
        self.palette.clear();
        self.palette_selected = 0;
        self.action_feedback = None;
        self.status_line = "Command palette".to_string();
    }

    pub fn dismiss_palette(&mut self) {
        self.focus = Focus::List;
        self.palette.clear();
        self.palette_selected = 0;
        if self.pending_confirmation.is_none() {
            self.status_line = "Ready".to_string();
            self.action_feedback = None;
        }
    }

    pub fn complete_palette(&mut self) {
        self.focus = Focus::List;
        self.palette.clear();
        self.palette_selected = 0;
    }

    pub fn next_tab(&mut self) {
        self.detail_tab = self.detail_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.detail_tab = self.detail_tab.previous();
    }

    pub fn request_refresh(&mut self) {
        self.invoke_action(ActionKind::RefreshState);
    }

    pub fn cycle_interface_visibility(&mut self) {
        self.set_interface_visibility(self.interface_visibility.next());
    }

    pub fn set_interface_visibility(&mut self, visibility: InterfaceVisibility) {
        self.interface_visibility = visibility;
        self.pending_confirmation = None;
        self.action_feedback = None;
        self.action_selected = 0;

        if let Some(first_visible) = self.visible_interface_indexes().first().copied() {
            self.selected = first_visible;
            if let Some(iface) = self.interfaces.get(first_visible) {
                self.status_line = format!(
                    "Showing {} interfaces, selected {}",
                    self.interface_visibility.title(),
                    iface.name
                );
            }
        } else {
            self.status_line = format!(
                "Showing {} interfaces, none available",
                self.interface_visibility.title()
            );
        }

        self.log.push(format!(
            "interface visibility set to {}",
            self.interface_visibility.title()
        ));
    }

    pub fn action_specs(&self) -> Vec<ActionSpec> {
        self.selected_interface()
            .map(action_catalog)
            .unwrap_or_default()
    }

    pub fn selected_action(&self) -> Option<ActionSpec> {
        let actions = self.action_specs();
        actions.get(self.action_selected).cloned()
    }

    pub fn next_action(&mut self) {
        let len = self.action_specs().len();
        if len > 0 {
            self.action_selected = (self.action_selected + 1) % len;
        }
    }

    pub fn previous_action(&mut self) {
        let len = self.action_specs().len();
        if len > 0 {
            self.action_selected = if self.action_selected == 0 {
                len - 1
            } else {
                self.action_selected - 1
            };
        }
    }

    pub fn invoke_selected_action(&mut self) {
        if let Some(spec) = self.selected_action() {
            self.invoke_action_spec(spec);
        }
    }

    pub fn invoke_action(&mut self, kind: ActionKind) {
        if let Some(iface) = self.selected_interface() {
            let spec = ActionSpec::for_interface(kind, iface);
            self.invoke_action_spec(spec);
        }
    }

    fn invoke_action_spec(&mut self, spec: ActionSpec) {
        let Some(iface) = self.selected_interface().cloned() else {
            return;
        };

        if matches!(spec.safety, crate::actions::ActionSafety::ConfirmRequired) {
            self.pending_confirmation = Some(pending_confirmation(spec.clone(), &iface));
            self.action_feedback = Some(ActionOutcome {
                headline: "Confirmation required".to_string(),
                detail: Some(format!(
                    "Review the prompt before running {} on {}.",
                    spec.title, iface.name
                )),
            });
            self.status_line = format!("Confirmation required for {}", spec.title);
            self.log.push(format!(
                "awaiting confirmation: {} on {}",
                spec.title, iface.name
            ));
            return;
        }

        self.apply_execution(execute_action(&spec, &iface));
    }

    pub fn confirm_pending_action(&mut self) {
        let Some(pending) = self.pending_confirmation.take() else {
            return;
        };

        let iface = self
            .selected_interface()
            .cloned()
            .filter(|iface| iface.name == pending.interface_name);

        if let Some(iface) = iface {
            self.log.push(format!(
                "confirmed action: {} on {}",
                pending.action.title, iface.name
            ));
            self.apply_execution(execute_action(&pending.action, &iface));
        } else {
            self.log.push(format!(
                "confirmation expired: {} target changed",
                pending.action.title
            ));
            self.status_line = "Confirmation expired".to_string();
            self.action_feedback = Some(ActionOutcome {
                headline: "Confirmation expired".to_string(),
                detail: Some(
                    "Selection changed before the gated action was confirmed.".to_string(),
                ),
            });
        }
    }

    pub fn cancel_pending_action(&mut self) {
        if let Some(pending) = self.pending_confirmation.take() {
            self.log.push(format!(
                "cancelled action: {} on {}",
                pending.action.title, pending.interface_name
            ));
            self.status_line = format!("Cancelled {}", pending.action.title);
            self.action_feedback = Some(ActionOutcome {
                headline: "Action cancelled".to_string(),
                detail: Some(format!(
                    "{} on {} was dismissed before execution.",
                    pending.action.title, pending.interface_name
                )),
            });
        }
    }

    fn apply_execution(&mut self, execution: crate::actions::ActionExecution) {
        self.log.push(execution.log_entry);
        self.status_line = execution.status_line;
        self.action_feedback = Some(execution.outcome);
        match execution.effect {
            ActionEffect::Refresh => {}
            ActionEffect::CopySummary => {}
            ActionEffect::FocusOverview => self.detail_tab = DetailTab::Overview,
            ActionEffect::Noop => {}
        }
    }

    pub fn select_next_palette_suggestion(&mut self) {
        let suggestions = self.filtered_palette_suggestions();
        if suggestions.is_empty() {
            self.palette_selected = 0;
            return;
        }

        self.palette_selected = (self.palette_selected + 1) % suggestions.len();
    }

    pub fn select_previous_palette_suggestion(&mut self) {
        let suggestions = self.filtered_palette_suggestions();
        if suggestions.is_empty() {
            self.palette_selected = 0;
            return;
        }

        self.palette_selected = if self.palette_selected == 0 {
            suggestions.len() - 1
        } else {
            self.palette_selected - 1
        };
    }

    pub fn apply_selected_palette_suggestion(&mut self) -> bool {
        let suggestions = self.filtered_palette_suggestions();
        let Some(command) = suggestions.get(self.palette_selected) else {
            return false;
        };

        self.palette = (*command).to_string();
        true
    }

    pub fn update_palette_input(&mut self, palette: String) {
        self.palette = palette;
        let suggestions = self.filtered_palette_suggestions();
        if suggestions.is_empty() {
            self.palette_selected = 0;
        } else if self.palette.trim().is_empty() {
            self.palette_selected = self.palette_selected.min(suggestions.len() - 1);
        } else {
            self.palette_selected = 0;
        }
    }

    pub fn execute_palette(&mut self) {
        let used_suggestion = self.palette.trim().is_empty();
        let command = if used_suggestion {
            self.filtered_palette_suggestions()
                .get(self.palette_selected)
                .copied()
                .unwrap_or("")
                .to_string()
        } else {
            self.palette.trim().to_lowercase()
        };

        if !command.is_empty() {
            self.palette_history.push(command.clone());
        }

        match command.as_str() {
            "refresh" | "reload" => self.invoke_action(ActionKind::RefreshState),
            "help" => {
                self.log.push(
                    "available commands: refresh, reload, show all, group inactive, show active, show inactive, help, copy, inspect, renew, quit"
                        .to_string(),
                );
                self.status_line = "Help opened in event log".to_string();
            }
            "show all" => self.set_interface_visibility(InterfaceVisibility::All),
            "group inactive" => self.set_interface_visibility(InterfaceVisibility::GroupInactive),
            "show active" => self.set_interface_visibility(InterfaceVisibility::ActiveOnly),
            "show inactive" => self.set_interface_visibility(InterfaceVisibility::InactiveOnly),
            "copy" => self.invoke_action(ActionKind::CopySummary),
            "inspect" => self.invoke_action(ActionKind::InspectServices),
            "renew" => self.invoke_action(ActionKind::RenewDhcpLease),
            "quit" | "exit" => self.should_quit = true,
            "" => {}
            other => {
                self.log.push(format!("unknown command: {other}"));
                self.status_line = format!("Unknown command: {other}");
            }
        }

        self.complete_palette();
    }

    pub fn shortcuts(&self) -> &'static str {
        "j/k move • v visibility • [/] details • a/s actions • enter run/confirm • esc cancel • p or : palette • q quit"
    }
}

fn palette_match_rank(command: &str, query: &str) -> Option<u8> {
    if query.is_empty() {
        return Some(0);
    }

    if command == query {
        Some(0)
    } else if command.starts_with(query) {
        Some(1)
    } else if command.contains(query) {
        Some(2)
    } else {
        None
    }
}
