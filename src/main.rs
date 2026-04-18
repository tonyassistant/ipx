mod app;
mod network;
mod tui;

use anyhow::Result;

fn main() -> Result<()> {
    let interfaces = network::load_interfaces();
    let mut app = app::App::new(interfaces);
    tui::run(&mut app)
}
