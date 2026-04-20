use crate::network::NetworkInterface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionSafety {
    ReadOnly,
    ConfirmRequired,
}

impl ActionSafety {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::ConfirmRequired => "confirm required",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    RefreshState,
    CopySummary,
    InspectServices,
    RenewDhcpLease,
}

impl ActionKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::RefreshState => "Refresh state",
            Self::CopySummary => "Copy summary",
            Self::InspectServices => "Inspect services",
            Self::RenewDhcpLease => "Renew DHCP lease",
        }
    }

    pub fn description(&self, iface: &NetworkInterface) -> String {
        match self {
            Self::RefreshState => format!("Reload {} telemetry and service posture", iface.name),
            Self::CopySummary => format!("Copy a concise {} summary for handoff", iface.device),
            Self::InspectServices => format!("Inspect mapped services for {}", iface.name),
            Self::RenewDhcpLease => format!(
                "Request a new DHCP lease for {}. This mutates live network state.",
                iface.name
            ),
        }
    }

    pub fn safety(&self) -> ActionSafety {
        match self {
            Self::RefreshState | Self::CopySummary | Self::InspectServices => {
                ActionSafety::ReadOnly
            }
            Self::RenewDhcpLease => ActionSafety::ConfirmRequired,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionSpec {
    pub kind: ActionKind,
    pub title: String,
    pub description: String,
    pub safety: ActionSafety,
    pub enabled: bool,
}

impl ActionSpec {
    pub fn for_interface(kind: ActionKind, iface: &NetworkInterface) -> Self {
        let enabled = !matches!(kind, ActionKind::RenewDhcpLease);
        Self {
            kind,
            title: kind.label().to_string(),
            description: kind.description(iface),
            safety: kind.safety(),
            enabled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingConfirmation {
    pub action: ActionSpec,
    pub interface_name: String,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionEffect {
    Refresh,
    CopySummary,
    FocusOverview,
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutcome {
    pub headline: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionExecution {
    pub log_entry: String,
    pub status_line: String,
    pub outcome: ActionOutcome,
    pub effect: ActionEffect,
}

pub fn action_catalog(iface: &NetworkInterface) -> Vec<ActionSpec> {
    [
        ActionKind::RefreshState,
        ActionKind::CopySummary,
        ActionKind::InspectServices,
        ActionKind::RenewDhcpLease,
    ]
    .into_iter()
    .map(|kind| ActionSpec::for_interface(kind, iface))
    .collect()
}

pub fn pending_confirmation(spec: ActionSpec, iface: &NetworkInterface) -> PendingConfirmation {
    PendingConfirmation {
        prompt: format!(
            "Confirm {} on {}? This is gated because it changes live network state.",
            spec.title, iface.name
        ),
        interface_name: iface.name.clone(),
        action: spec,
    }
}

pub fn execute_action(spec: &ActionSpec, iface: &NetworkInterface) -> ActionExecution {
    match spec.kind {
        ActionKind::RefreshState => ActionExecution {
            log_entry: format!("refresh requested for {}", iface.name),
            status_line: format!("Refresh requested for {}", iface.name),
            outcome: ActionOutcome {
                headline: "Refresh queued".to_string(),
                detail: Some(format!(
                    "{} remains selected while telemetry reloads.",
                    iface.name
                )),
            },
            effect: ActionEffect::Refresh,
        },
        ActionKind::CopySummary => ActionExecution {
            log_entry: format!("copied summary for {}", iface.device),
            status_line: format!("Copied {} summary", iface.device),
            outcome: ActionOutcome {
                headline: "Summary copied".to_string(),
                detail: Some(format!(
                    "{} is ready to paste into handoff notes.",
                    iface.device
                )),
            },
            effect: ActionEffect::CopySummary,
        },
        ActionKind::InspectServices => ActionExecution {
            log_entry: format!("inspecting services for {}", iface.name),
            status_line: format!("Inspecting {} services", iface.name),
            outcome: ActionOutcome {
                headline: "Service inspection opened".to_string(),
                detail: Some(format!(
                    "Review mapped services and notes for {} in Overview.",
                    iface.name
                )),
            },
            effect: ActionEffect::FocusOverview,
        },
        ActionKind::RenewDhcpLease => ActionExecution {
            log_entry: format!(
                "blocked mutating action for {}: DHCP renew stays disabled in v1",
                iface.name
            ),
            status_line: "Mutating actions stay disabled in v1".to_string(),
            outcome: ActionOutcome {
                headline: "Live network change blocked".to_string(),
                detail: Some("DHCP renew stays confirmation-gated and disabled in v1.".to_string()),
            },
            effect: ActionEffect::Noop,
        },
    }
}
