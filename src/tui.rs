use std::{io, time::Duration};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::app::{App, Focus};

pub fn run(app: &mut App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| draw(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.focus {
                        Focus::List => match key.code {
                            KeyCode::Char('q') => app.should_quit = true,
                            KeyCode::Char('j') | KeyCode::Down => app.next(),
                            KeyCode::Char('k') | KeyCode::Up => app.previous(),
                            KeyCode::Char('p') | KeyCode::Char(':') => app.open_palette(),
                            KeyCode::Char('r') => app.log.push("refresh requested".to_string()),
                            _ => {}
                        },
                        Focus::Palette => match key.code {
                            KeyCode::Esc => app.close_palette(),
                            KeyCode::Enter => app.execute_palette(),
                            KeyCode::Backspace => {
                                app.palette.pop();
                            }
                            KeyCode::Char(c) => app.palette.push(c),
                            _ => {}
                        },
                    }
                }
            }
        }
    }

    Ok(())
}

fn draw(frame: &mut Frame, app: &App) {
    let area = frame.size();
    let vertical = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(8),
        Constraint::Length(2),
    ])
    .split(area);

    let header = Paragraph::new("ipx  v1  •  macOS network operations").block(
        Block::default()
            .borders(Borders::ALL)
            .title("Operator Console"),
    );
    frame.render_widget(header, vertical[0]);

    let middle = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(vertical[1]);

    let items: Vec<ListItem> = app
        .interfaces
        .iter()
        .enumerate()
        .map(|(idx, iface)| {
            let marker = if idx == app.selected { "›" } else { " " };
            ListItem::new(format!(
                "{marker} {} ({}) [{}]",
                iface.name, iface.device, iface.status
            ))
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Interfaces"));
    frame.render_widget(list, middle[0]);

    let detail_text = if let Some(iface) = app.selected_interface() {
        format!(
            "Name: {}\nDevice: {}\nKind: {}\nStatus: {}\nIPv4: {}\nMAC: {}",
            iface.name,
            iface.device,
            iface.kind,
            iface.status,
            iface.ipv4.as_deref().unwrap_or("-"),
            iface.mac.as_deref().unwrap_or("-")
        )
    } else {
        "No interface selected".to_string()
    };

    let details = Paragraph::new(detail_text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Inspector"));
    frame.render_widget(details, middle[1]);

    let log = Paragraph::new(
        app.log
            .iter()
            .rev()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .block(Block::default().borders(Borders::ALL).title("Event Log"))
    .wrap(Wrap { trim: true });
    frame.render_widget(log, vertical[2]);

    let footer = Paragraph::new("j/k move • r refresh • p or : palette • q quit")
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, vertical[3]);

    if app.focus == Focus::Palette {
        let popup = centered_rect(70, 18, area);
        frame.render_widget(Clear, popup);
        let palette = Paragraph::new(app.palette.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Command Palette"),
        );
        frame.render_widget(palette, popup);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
