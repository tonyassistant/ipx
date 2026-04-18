use crate::network::NetworkInterface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Palette,
}

#[derive(Debug)]
pub struct App {
    pub interfaces: Vec<NetworkInterface>,
    pub selected: usize,
    pub focus: Focus,
    pub palette: String,
    pub log: Vec<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(interfaces: Vec<NetworkInterface>) -> Self {
        Self {
            interfaces,
            selected: 0,
            focus: Focus::List,
            palette: String::new(),
            log: vec!["ipx initialized".to_string()],
            should_quit: false,
        }
    }

    pub fn next(&mut self) {
        if !self.interfaces.is_empty() {
            self.selected = (self.selected + 1) % self.interfaces.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.interfaces.is_empty() {
            self.selected = if self.selected == 0 {
                self.interfaces.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn selected_interface(&self) -> Option<&NetworkInterface> {
        self.interfaces.get(self.selected)
    }

    pub fn open_palette(&mut self) {
        self.focus = Focus::Palette;
        self.palette.clear();
    }

    pub fn close_palette(&mut self) {
        self.focus = Focus::List;
        self.palette.clear();
    }

    pub fn execute_palette(&mut self) {
        let command = self.palette.trim().to_lowercase();
        match command.as_str() {
            "refresh" | "reload" => self.log.push("refresh requested".to_string()),
            "help" => self
                .log
                .push("available commands: refresh, reload, help".to_string()),
            "quit" | "exit" => self.should_quit = true,
            "" => {}
            other => self.log.push(format!("unknown command: {other}")),
        }
        self.close_palette();
    }
}
