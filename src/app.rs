use crate::network::NetworkInterface;

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
        }
    }

    pub fn next(&mut self) {
        if !self.interfaces.is_empty() {
            self.selected = (self.selected + 1) % self.interfaces.len();
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

    pub fn open_palette(&mut self) {
        self.focus = Focus::Palette;
        self.palette.clear();
        self.status_line = "Command palette".to_string();
    }

    pub fn close_palette(&mut self) {
        self.focus = Focus::List;
        self.palette.clear();
        self.status_line = "Ready".to_string();
    }

    pub fn next_tab(&mut self) {
        self.detail_tab = self.detail_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.detail_tab = self.detail_tab.previous();
    }

    pub fn request_refresh(&mut self) {
        self.log.push("refresh requested".to_string());
        self.status_line = "Refresh requested".to_string();
    }

    pub fn execute_palette(&mut self) {
        let command = self.palette.trim().to_lowercase();
        if !command.is_empty() {
            self.palette_history.push(command.clone());
        }

        match command.as_str() {
            "refresh" | "reload" => self.request_refresh(),
            "help" => {
                self.log.push(
                    "available commands: refresh, reload, help, copy, inspect, quit".to_string(),
                );
                self.status_line = "Help opened in event log".to_string();
            }
            "copy" => {
                self.log
                    .push("copy requested for selected interface".to_string());
                self.status_line = "Copy action queued".to_string();
            }
            "inspect" => {
                self.detail_tab = DetailTab::Overview;
                self.status_line = "Inspector focused".to_string();
            }
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
        "j/k move • tab cycle pane • [/] cycle details • r refresh • p or : palette • q quit"
    }
}
