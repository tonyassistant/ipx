use crate::{
    actions::{
        action_catalog, execute_action, pending_confirmation, ActionEffect, ActionKind, ActionSpec,
        PendingConfirmation,
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
    pub action_selected: usize,
    pub pending_confirmation: Option<PendingConfirmation>,
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
            action_selected: 0,
            pending_confirmation: None,
        }
    }

    pub fn next(&mut self) {
        if !self.interfaces.is_empty() {
            self.selected = (self.selected + 1) % self.interfaces.len();
            self.action_selected = 0;
            self.status_line = format!(
                "Selected {}",
                self.selected_interface()
                    .map(|i| i.name.clone())
                    .unwrap_or_default()
            );
        }
    }

    pub fn previous(&mut self) {
        if !self.interfaces.is_empty() {
            self.selected = if self.selected == 0 {
                self.interfaces.len() - 1
            } else {
                self.selected - 1
            };
            self.action_selected = 0;
            self.status_line = format!(
                "Selected {}",
                self.selected_interface()
                    .map(|i| i.name.clone())
                    .unwrap_or_default()
            );
        }
    }

    pub fn selected_interface(&self) -> Option<&NetworkInterface> {
        self.interfaces.get(self.selected)
    }

    pub fn selection_label(&self) -> String {
        if self.interfaces.is_empty() {
            "No interfaces".to_string()
        } else {
            format!("{}/{}", self.selected + 1, self.interfaces.len())
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
            "refresh", "reload", "help", "copy", "inspect", "renew", "quit",
        ]
    }

    pub fn open_palette(&mut self) {
        self.focus = Focus::Palette;
        self.palette.clear();
        self.status_line = "Command palette".to_string();
    }

    pub fn close_palette(&mut self) {
        self.focus = Focus::List;
        self.palette.clear();
        if self.pending_confirmation.is_none() {
            self.status_line = "Ready".to_string();
        }
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
        }
    }

    pub fn cancel_pending_action(&mut self) {
        if let Some(pending) = self.pending_confirmation.take() {
            self.log.push(format!(
                "cancelled action: {} on {}",
                pending.action.title, pending.interface_name
            ));
            self.status_line = format!("Cancelled {}", pending.action.title);
        }
    }

    fn apply_execution(&mut self, execution: crate::actions::ActionExecution) {
        self.log.push(execution.log_entry);
        self.status_line = execution.status_line;
        match execution.effect {
            ActionEffect::Refresh => {}
            ActionEffect::CopySummary => {}
            ActionEffect::FocusOverview => self.detail_tab = DetailTab::Overview,
            ActionEffect::Noop => {}
        }
    }

    pub fn execute_palette(&mut self) {
        let command = self.palette.trim().to_lowercase();
        if !command.is_empty() {
            self.palette_history.push(command.clone());
        }

        match command.as_str() {
            "refresh" | "reload" => self.invoke_action(ActionKind::RefreshState),
            "help" => {
                self.log.push(
                    "available commands: refresh, reload, help, copy, inspect, renew, quit"
                        .to_string(),
                );
                self.status_line = "Help opened in event log".to_string();
            }
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
        self.close_palette();
    }

    pub fn shortcuts(&self) -> &'static str {
        "j/k move • [/] details • a/s actions • enter run/confirm • esc cancel • p or : palette • q quit"
    }
}
