use std::{io, time::Duration};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
};

use crate::{
    actions::ActionSafety,
    app::{App, DetailTab, Focus},
    network::{InterfaceStatus, NetworkInterface, ReachabilityState},
};

const OPERATOR_AMBER: Color = Color::Rgb(255, 191, 87);
const OPERATOR_SURFACE: Color = Color::Rgb(24, 30, 42);
const OPERATOR_MUTED: Color = Color::Rgb(120, 134, 156);

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
                            KeyCode::Char('r') => app.request_refresh(),
                            KeyCode::Char('v') => app.cycle_interface_visibility(),
                            KeyCode::Char('a') if app.detail_tab == DetailTab::Actions => {
                                app.previous_action()
                            }
                            KeyCode::Char('s') if app.detail_tab == DetailTab::Actions => {
                                app.next_action()
                            }
                            KeyCode::Enter => {
                                if app.pending_confirmation.is_some() {
                                    app.confirm_pending_action();
                                } else if app.detail_tab == DetailTab::Actions {
                                    app.invoke_selected_action();
                                }
                            }
                            KeyCode::Esc if app.pending_confirmation.is_some() => {
                                app.cancel_pending_action()
                            }
                            KeyCode::Tab | KeyCode::Char(']') => app.next_tab(),
                            KeyCode::BackTab | KeyCode::Char('[') => app.previous_tab(),
                            _ => {}
                        },
                        Focus::Palette => match key.code {
                            KeyCode::Esc => app.dismiss_palette(),
                            KeyCode::Enter => app.execute_palette(),
                            KeyCode::Tab => {
                                if !app.apply_selected_palette_suggestion() {
                                    app.select_next_palette_suggestion();
                                }
                            }
                            KeyCode::Down => app.select_next_palette_suggestion(),
                            KeyCode::BackTab | KeyCode::Up => {
                                app.select_previous_palette_suggestion()
                            }
                            KeyCode::Right => {
                                let _ = app.apply_selected_palette_suggestion();
                            }
                            KeyCode::Backspace => {
                                let mut next = app.palette.clone();
                                next.pop();
                                app.update_palette_input(next);
                            }
                            KeyCode::Char(c) => {
                                let mut next = app.palette.clone();
                                next.push(c);
                                app.update_palette_input(next);
                            }
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
        Constraint::Length(4),
        Constraint::Min(16),
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Length(2),
    ])
    .split(area);

    draw_header(frame, app, vertical[0]);

    let middle = Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(vertical[1]);

    draw_interface_list(frame, app, middle[0]);
    draw_inspector(frame, app, middle[1]);
    draw_action_feedback(frame, app, vertical[2]);
    draw_log(frame, app, vertical[3]);
    draw_footer(frame, app, vertical[4]);

    if app.focus == Focus::Palette {
        draw_palette(frame, app, area);
    }

    if app.pending_confirmation.is_some() {
        draw_confirmation(frame, app, area);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let counts = app.interface_counts();
    let header = Paragraph::new(vec![
        Line::from(vec![
            " ipx ".black().on_cyan().bold(),
            " operator console ".fg(Color::Cyan).bold(),
            "macOS network surfaces".fg(OPERATOR_MUTED),
        ]),
        Line::from(vec![
            metric_span("UP", counts.connected, Color::Green),
            "  ".into(),
            metric_span("DEGRADED", counts.disconnected, OPERATOR_AMBER),
            "  ".into(),
            metric_span("STANDBY", counts.inactive, OPERATOR_MUTED),
            "  ".into(),
            Span::styled(
                format!("FOCUS {}", focus_label(app.focus)),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            "  ".into(),
            Span::styled(
                format!("SELECTION {}", app.selection_label()),
                Style::default().fg(Color::White),
            ),
            "  ".into(),
            Span::styled(
                format!(
                    "VISIBILITY {}",
                    app.interface_visibility.title().to_uppercase()
                ),
                Style::default().fg(OPERATOR_MUTED),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Control Deck")
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(OPERATOR_SURFACE)),
    );
    frame.render_widget(header, area);
}

fn draw_interface_list(frame: &mut Frame, app: &App, area: Rect) {
    let visible = app.visible_interfaces();
    let (primary, inactive) = app.grouped_interfaces();
    let items: Vec<ListItem> = if visible.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "No interfaces match the current visibility filter",
            Style::default().fg(OPERATOR_MUTED),
        )))]
    } else if app.interface_visibility == crate::app::InterfaceVisibility::GroupInactive {
        let mut items = Vec::new();

        if !primary.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "Active and degraded",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ))));
            items.extend(
                primary
                    .iter()
                    .map(|(idx, iface)| ListItem::new(interface_row(iface, *idx == app.selected))),
            );
        }

        if !inactive.is_empty() {
            if !items.is_empty() {
                items.push(ListItem::new(Line::from("")));
            }
            items.push(ListItem::new(Line::from(Span::styled(
                format!("Inactive ({})", inactive.len()),
                Style::default()
                    .fg(OPERATOR_MUTED)
                    .add_modifier(Modifier::BOLD),
            ))));
            items.extend(
                inactive
                    .iter()
                    .map(|(idx, iface)| ListItem::new(interface_row(iface, *idx == app.selected))),
            );
        }

        items
    } else {
        visible
            .iter()
            .map(|(idx, iface)| ListItem::new(interface_row(iface, *idx == app.selected)))
            .collect()
    };

    let title = format!(
        "Interfaces [{} visible / {} total]",
        visible.len(),
        app.interfaces.len()
    );
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(focus_border(app.focus == Focus::List)),
    );
    frame.render_widget(list, area);
}

fn draw_inspector(frame: &mut Frame, app: &App, area: Rect) {
    let detail_chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(8)]).split(area);
    let titles = [DetailTab::Overview, DetailTab::Signals, DetailTab::Actions]
        .iter()
        .map(|tab| Line::from(tab.title()))
        .collect::<Vec<_>>();
    let selected_tab = match app.detail_tab {
        DetailTab::Overview => 0,
        DetailTab::Signals => 1,
        DetailTab::Actions => 2,
    };
    let tabs = Tabs::new(titles)
        .select(selected_tab)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider("│")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Inspector")
                .border_style(Style::default().fg(Color::Cyan)),
        );
    frame.render_widget(tabs, detail_chunks[0]);

    let details = Paragraph::new(detail_lines(app))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(detail_title(app))
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(details, detail_chunks[1]);
}

fn draw_action_feedback(frame: &mut Frame, app: &App, area: Rect) {
    let lines = if let Some(feedback) = &app.action_feedback {
        let mut lines = vec![Line::from(vec![Span::styled(
            feedback.headline.as_str(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )])];

        if let Some(detail) = &feedback.detail {
            lines.push(Line::from(Span::styled(
                detail.as_str(),
                Style::default().fg(Color::White),
            )));
        }

        lines
    } else {
        vec![Line::from(Span::styled(
            "No recent action feedback",
            Style::default().fg(OPERATOR_MUTED),
        ))]
    };

    let feedback = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Action Feedback")
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(feedback, area);
}

fn draw_log(frame: &mut Frame, app: &App, area: Rect) {
    let lines = app
        .log
        .iter()
        .rev()
        .take(5)
        .enumerate()
        .map(|(idx, entry)| {
            Line::from(vec![
                Span::styled(
                    format!("{:>2}", idx + 1),
                    Style::default()
                        .fg(OPERATOR_MUTED)
                        .add_modifier(Modifier::BOLD),
                ),
                " ".into(),
                Span::raw(entry.clone()),
            ])
        })
        .collect::<Vec<_>>();

    let log = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Event Log")
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(log, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(app.shortcuts(), Style::default().fg(OPERATOR_MUTED)),
        "  •  ".into(),
        Span::styled(
            app.status_line.as_str(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, area);
}

fn draw_palette(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(72, 30, area);
    frame.render_widget(Clear, popup);

    let sections = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Min(3),
    ])
    .margin(1)
    .split(popup);

    let shell = Block::default()
        .borders(Borders::ALL)
        .title("Command Palette")
        .border_style(focus_border(true))
        .style(Style::default().bg(OPERATOR_SURFACE));
    frame.render_widget(shell, popup);

    let input = Paragraph::new(Line::from(vec![
        Span::styled(
            "> ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(app.palette.as_str()),
    ]));
    frame.render_widget(input, sections[0]);

    let hint = Paragraph::new("enter run • esc cancel").style(Style::default().fg(OPERATOR_MUTED));
    frame.render_widget(hint, sections[1]);

    let suggestions = app.filtered_palette_suggestions();
    let suggestion_lines = if suggestions.is_empty() {
        vec![Line::from(Span::styled(
            "No matching commands",
            Style::default().fg(OPERATOR_MUTED),
        ))]
    } else {
        suggestions
            .iter()
            .enumerate()
            .map(|(idx, command)| {
                let active = idx == app.palette_selected;
                Line::from(vec![
                    Span::styled(
                        if active { "▶" } else { "•" },
                        Style::default().fg(if active { Color::Cyan } else { OPERATOR_MUTED }),
                    ),
                    " ".into(),
                    Span::styled(
                        *command,
                        Style::default()
                            .fg(if active { Color::White } else { OPERATOR_MUTED })
                            .add_modifier(if active {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                    ),
                ])
            })
            .collect::<Vec<_>>()
    };
    frame.render_widget(Paragraph::new(suggestion_lines), sections[2]);
}

fn interface_row(iface: &NetworkInterface, selected: bool) -> Line<'static> {
    let marker = if selected { "▶" } else { " " };
    let style = status_style(&iface.status);
    let meta_style = Style::default().fg(OPERATOR_MUTED);

    Line::from(vec![
        Span::styled(marker.to_string(), Style::default().fg(Color::Cyan)),
        " ".into(),
        Span::styled(iface.name.clone(), style.add_modifier(Modifier::BOLD)),
        " ".into(),
        Span::styled(format!("{}", iface.device), meta_style),
        "  ".into(),
        Span::styled(iface.status.label().to_uppercase(), style),
    ])
}

fn detail_title(app: &App) -> String {
    let tab = match app.detail_tab {
        DetailTab::Overview => "Overview",
        DetailTab::Signals => "Signal Watch",
        DetailTab::Actions => "Action Queue",
    };

    app.selected_interface()
        .map(|iface| format!("{tab} • {}", iface.name))
        .unwrap_or_else(|| tab.to_string())
}

fn detail_lines(app: &App) -> Vec<Line<'static>> {
    match app.detail_tab {
        DetailTab::Overview => app
            .selected_interface()
            .map(overview_lines)
            .unwrap_or_else(|| vec![Line::from("No interface selected")]),
        DetailTab::Signals => app
            .selected_interface()
            .map(signal_lines)
            .unwrap_or_else(|| vec![Line::from("No interface selected")]),
        DetailTab::Actions => app
            .selected_interface()
            .map(|iface| action_lines(app, iface))
            .unwrap_or_else(|| vec![Line::from("No interface selected")]),
    }
}

fn overview_lines(iface: &NetworkInterface) -> Vec<Line<'static>> {
    let mut lines = vec![
        kv_line("Name", iface.name.clone()),
        kv_line("Device", iface.device.clone()),
        kv_line("Kind", iface.kind.label().to_string()),
        kv_line("Status", iface.status.label().to_string()),
        styled_kv_line(
            "Reach",
            iface.reachability().label().to_string(),
            reachability_style(iface.reachability()),
        ),
        kv_line(
            "IPv4",
            iface.ipv4.clone().unwrap_or_else(|| "-".to_string()),
        ),
        kv_line("MAC", iface.mac.clone().unwrap_or_else(|| "-".to_string())),
        Line::from(""),
        section_line("Services"),
    ];

    if iface.services.is_empty() {
        lines.push(bullet_line("No mapped services discovered".to_string()));
    } else {
        lines.extend(
            iface
                .services
                .iter()
                .map(|service| bullet_line(service.summary())),
        );
    }

    if !iface.notes.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_line("Notes"));
        lines.extend(iface.notes.iter().cloned().map(bullet_line));
    }

    lines
}

fn signal_lines(iface: &NetworkInterface) -> Vec<Line<'static>> {
    let reachability = iface.reachability();

    vec![
        section_line("Live posture"),
        bullet_line(format!("Link state {}", iface.status.label())),
        bullet_line(format!("Reachability {}", reachability.label())),
        bullet_line(reachability.note().to_string()),
        bullet_line(format!(
            "Address {}",
            iface.ipv4.as_deref().unwrap_or("unassigned")
        )),
        bullet_line(format!(
            "Gateway {}",
            iface.gateway.as_deref().unwrap_or("unassigned")
        )),
        bullet_line(format!(
            "Default route {}",
            iface
                .default_route
                .as_ref()
                .map(|route| route.summary())
                .unwrap_or_else(|| "unassigned".to_string())
        )),
        bullet_line(format!("Service bindings {}", iface.services.len())),
        Line::from(""),
        section_line("Next parser pass"),
        bullet_line("RSSI and noise floor for Wi-Fi surfaces".to_string()),
        bullet_line("Negotiated speed and duplex for wired links".to_string()),
        bullet_line("Active probe-backed reachability validation after route modeling".to_string()),
    ]
}

fn action_lines(app: &App, iface: &NetworkInterface) -> Vec<Line<'static>> {
    let mut lines = vec![section_line("Safe actions")];

    for (idx, action) in app.action_specs().into_iter().enumerate() {
        let marker = if idx == app.action_selected {
            "▶"
        } else {
            "•"
        };
        let suffix = if action.enabled {
            action.safety.label().to_string()
        } else {
            format!("{} • blocked in v1", action.safety.label())
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{marker} "), Style::default().fg(Color::Cyan)),
            Span::styled(action.title, Style::default().fg(Color::White)),
            Span::styled(format!(" ({suffix})"), Style::default().fg(OPERATOR_MUTED)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(action.description, Style::default().fg(OPERATOR_MUTED)),
        ]));
    }

    lines.extend([
        Line::from(""),
        section_line("Guardrails"),
        bullet_line(format!("Read-first workflow anchored on {}", iface.name)),
        bullet_line("Read-only actions execute immediately".to_string()),
        bullet_line("Mutating actions require explicit confirmation".to_string()),
        bullet_line("Live network changes remain disabled in v1".to_string()),
    ]);

    lines
}

fn kv_line(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:>8} "),
            Style::default()
                .fg(OPERATOR_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(value, Style::default().fg(Color::White)),
    ])
}

fn styled_kv_line(label: &str, value: String, value_style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:>8} "),
            Style::default()
                .fg(OPERATOR_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(value, value_style),
    ])
}

fn section_line(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

fn bullet_line(text: String) -> Line<'static> {
    Line::from(vec![
        Span::styled("• ", Style::default().fg(Color::Cyan)),
        Span::raw(text),
    ])
}

fn metric_span(label: &str, value: usize, color: Color) -> Span<'static> {
    Span::styled(
        format!("{label} {value}"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

fn focus_label(focus: Focus) -> &'static str {
    match focus {
        Focus::List => "LIST",
        Focus::Palette => "PALETTE",
    }
}

fn focus_border(active: bool) -> Style {
    if active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn status_style(status: &InterfaceStatus) -> Style {
    match status {
        InterfaceStatus::Connected => Style::default().fg(Color::Green),
        InterfaceStatus::Disconnected => Style::default().fg(OPERATOR_AMBER),
        InterfaceStatus::Inactive => Style::default().fg(OPERATOR_MUTED),
    }
}

fn reachability_style(state: ReachabilityState) -> Style {
    match state {
        ReachabilityState::Reachable => Style::default().fg(Color::Green),
        ReachabilityState::LocalOnly => Style::default().fg(OPERATOR_AMBER),
        ReachabilityState::Down | ReachabilityState::Unknown => Style::default().fg(OPERATOR_MUTED),
    }
}

fn draw_confirmation(frame: &mut Frame, app: &App, area: Rect) {
    let Some(pending) = &app.pending_confirmation else {
        return;
    };

    let popup = centered_rect(64, 26, area);
    frame.render_widget(Clear, popup);

    let sections = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .margin(1)
    .split(popup);

    let shell = Block::default()
        .borders(Borders::ALL)
        .title("Confirmation Gate")
        .border_style(Style::default().fg(OPERATOR_AMBER))
        .style(Style::default().bg(OPERATOR_SURFACE));
    frame.render_widget(shell, popup);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Action ", Style::default().fg(OPERATOR_MUTED)),
            Span::styled(
                pending.action.title.as_str(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ])),
        sections[0],
    );

    frame.render_widget(
        Paragraph::new(pending.prompt.as_str()).wrap(Wrap { trim: true }),
        sections[1],
    );

    let safety = match pending.action.safety {
        ActionSafety::ReadOnly => "read-only",
        ActionSafety::ConfirmRequired => "confirm required",
    };
    frame.render_widget(
        Paragraph::new(vec![
            bullet_line(format!("Target {}", pending.interface_name)),
            bullet_line(format!("Safety posture {safety}")),
            bullet_line(if pending.action.enabled {
                "Execution available".to_string()
            } else {
                "Execution remains blocked in v1 after confirmation".to_string()
            }),
        ]),
        sections[2],
    );

    frame.render_widget(
        Paragraph::new("enter confirm • esc cancel").style(Style::default().fg(OPERATOR_MUTED)),
        sections[3],
    );
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

#[cfg(test)]
mod tests {
    use super::{detail_title, focus_label};
    use crate::{
        app::{App, Focus},
        network::sample_interfaces,
    };

    #[test]
    fn detail_title_includes_selected_interface_name() {
        let app = App::new(sample_interfaces());
        assert_eq!(detail_title(&app), "Overview • Wi-Fi");
    }

    #[test]
    fn focus_label_matches_focus_mode() {
        assert_eq!(focus_label(Focus::List), "LIST");
        assert_eq!(focus_label(Focus::Palette), "PALETTE");
    }
}
