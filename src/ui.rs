//! UI rendering. One public function — `draw` — which takes the current
//! `App` and paints every widget for the frame. The design aims for a
//! calm, terminal-native feel: muted borders, a single accent color for
//! the active tab and selection, and dense information without clutter.

use crate::app::{App, Tab};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const WARN: Color = Color::Yellow;
const ERR: Color = Color::Red;
const OK: Color = Color::Green;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tabs
            Constraint::Min(5),    // content
            Constraint::Length(1), // status
            Constraint::Length(1), // help
        ])
        .split(area);

    draw_tabs(frame, chunks[0], app);

    match app.tab {
        Tab::BaseDirs => draw_base_dirs(frame, chunks[1], app),
        Tab::MimeTypes => draw_mime(frame, chunks[1], app),
        Tab::Applications => draw_apps(frame, chunks[1], app),
    }

    draw_status(frame, chunks[2], app);
    draw_help(frame, chunks[3], app);
}

fn draw_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| Line::from(format!(" {} ", t.title())))
        .collect();

    let selected = Tab::ALL.iter().position(|t| *t == app.tab).unwrap_or(0);

    let tabs = Tabs::new(titles)
        .select(selected)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .title(Span::styled(
                    " xdg-tui ",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                )),
        )
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .divider("│");

    frame.render_widget(tabs, area);
}

fn draw_base_dirs(frame: &mut Frame, area: Rect, app: &mut App) {
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let items: Vec<ListItem> = app
        .base_dirs
        .dirs
        .iter()
        .map(|d| {
            let marker = if d.from_env { "●" } else { "○" };
            let marker_color = if d.from_env { ACCENT } else { DIM };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", marker), Style::default().fg(marker_color)),
                Span::styled(d.name, Style::default().add_modifier(Modifier::BOLD)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .title(" Variables "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(30, 50, 60))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, split[0], &mut app.base_state);

    let selected = app.base_state.selected().unwrap_or(0);
    if let Some(dir) = app.base_dirs.dirs.get(selected) {
        let exists_span = if dir.exists {
            Span::styled("exists", Style::default().fg(OK))
        } else {
            Span::styled("missing", Style::default().fg(WARN))
        };
        let source_span = if dir.from_env {
            Span::styled("from environment", Style::default().fg(ACCENT))
        } else {
            Span::styled("spec default", Style::default().fg(DIM))
        };

        let lines = vec![
            Line::from(vec![
                Span::styled(dir.name, Style::default().fg(ACCENT).bold()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Path      ", Style::default().fg(DIM)),
                Span::raw(dir.value.display().to_string()),
            ]),
            Line::from(vec![
                Span::styled("Source    ", Style::default().fg(DIM)),
                source_span,
            ]),
            Line::from(vec![
                Span::styled("Status    ", Style::default().fg(DIM)),
                exists_span,
            ]),
            Line::from(""),
            Line::from(Span::styled(dir.description, Style::default().italic())),
        ];

        let detail = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DIM))
                    .title(" Detail "),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(detail, split[1]);
    }
}

fn draw_mime(frame: &mut Frame, area: Rect, app: &mut App) {
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let indices = app.filtered_mime_indices();

    let filter_line = if app.mime_filter_active || !app.mime_filter.is_empty() {
        format!(" filter: {}_ ", app.mime_filter)
    } else {
        String::from(" / to filter ")
    };
    let filter_style = if app.mime_filter_active {
        Style::default().fg(ACCENT).bold()
    } else {
        Style::default().fg(DIM)
    };

    let items: Vec<ListItem> = indices
        .iter()
        .map(|&i| {
            let m = &app.mime_assocs[i];
            let default = m.default_app.as_deref().unwrap_or("—");
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {:<36} ", truncate(&m.mime_type, 36)), Style::default().fg(Color::Gray)),
                Span::styled(truncate(default, 28), Style::default().fg(ACCENT)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .title(Line::from(vec![
                    Span::raw(" MIME → Default "),
                    Span::styled(format!("({}) ", indices.len()), Style::default().fg(DIM)),
                    Span::styled(filter_line, filter_style),
                ])),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(30, 50, 60))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, split[0], &mut app.mime_state);

    // Detail panel
    let selected = app.mime_state.selected().unwrap_or(0);
    let detail_lines: Vec<Line> = if let Some(&real_idx) = indices.get(selected) {
        let m = &app.mime_assocs[real_idx];
        let mut lines = vec![
            Line::from(Span::styled(&m.mime_type, Style::default().fg(ACCENT).bold())),
            Line::from(""),
            Line::from(vec![
                Span::styled("Default   ", Style::default().fg(DIM)),
                Span::raw(m.default_app.clone().unwrap_or_else(|| "(none)".into())),
            ]),
        ];
        if !m.added_apps.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Added associations",
                Style::default().fg(DIM).italic(),
            )));
            for a in &m.added_apps {
                lines.push(Line::from(format!("  • {}", a)));
            }
        }
        if !m.removed_apps.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Removed associations",
                Style::default().fg(DIM).italic(),
            )));
            for r in &m.removed_apps {
                lines.push(Line::from(format!("  • {}", r)));
            }
        }
        lines
    } else {
        vec![Line::from(Span::styled(
            "No MIME associations found. Your mimeapps.list may be empty or absent.",
            Style::default().fg(DIM).italic(),
        ))]
    };

    let detail = Paragraph::new(detail_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .title(" Detail "),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(detail, split[1]);
}

fn draw_apps(frame: &mut Frame, area: Rect, app: &mut App) {
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let indices = app.filtered_app_indices();

    let filter_line = if app.app_filter_active || !app.app_filter.is_empty() {
        format!(" filter: {}_ ", app.app_filter)
    } else {
        String::from(" / to filter ")
    };
    let filter_style = if app.app_filter_active {
        Style::default().fg(ACCENT).bold()
    } else {
        Style::default().fg(DIM)
    };

    let items: Vec<ListItem> = indices
        .iter()
        .map(|&i| {
            let a = &app.apps[i];
            let hidden = if a.no_display { "·" } else { " " };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", hidden), Style::default().fg(DIM)),
                Span::raw(truncate(&a.name, 36)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .title(Line::from(vec![
                    Span::raw(" Applications "),
                    Span::styled(format!("({}) ", indices.len()), Style::default().fg(DIM)),
                    Span::styled(filter_line, filter_style),
                ])),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(30, 50, 60))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, split[0], &mut app.app_state);

    let selected = app.app_state.selected().unwrap_or(0);
    let detail = if let Some(&real_idx) = indices.get(selected) {
        let a = &app.apps[real_idx];
        let mut lines = vec![
            Line::from(Span::styled(&a.name, Style::default().fg(ACCENT).bold())),
        ];
        if let Some(ref g) = a.generic_name {
            lines.push(Line::from(Span::styled(
                g.clone(),
                Style::default().fg(DIM).italic(),
            )));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("ID        ", Style::default().fg(DIM)),
            Span::raw(a.id.clone()),
        ]));
        if let Some(ref e) = a.exec {
            lines.push(Line::from(vec![
                Span::styled("Exec      ", Style::default().fg(DIM)),
                Span::raw(truncate(e, 60).to_string()),
            ]));
        }
        if let Some(ref i) = a.icon {
            lines.push(Line::from(vec![
                Span::styled("Icon      ", Style::default().fg(DIM)),
                Span::raw(i.clone()),
            ]));
        }
        lines.push(Line::from(vec![
            Span::styled("Terminal  ", Style::default().fg(DIM)),
            Span::raw(a.terminal.to_string()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("NoDisplay ", Style::default().fg(DIM)),
            Span::raw(a.no_display.to_string()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Path      ", Style::default().fg(DIM)),
            Span::raw(a.path.display().to_string()),
        ]));
        if let Some(ref c) = a.comment {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::raw(c.clone())));
        }
        if !a.categories.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Categories",
                Style::default().fg(DIM).italic(),
            )));
            lines.push(Line::from(a.categories.join(" · ")));
        }
        if !a.mime_types.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("Handles {} MIME type(s)", a.mime_types.len()),
                Style::default().fg(DIM).italic(),
            )));
            for m in a.mime_types.iter().take(8) {
                lines.push(Line::from(format!("  • {}", m)));
            }
            if a.mime_types.len() > 8 {
                lines.push(Line::from(Span::styled(
                    format!("  …and {} more", a.mime_types.len() - 8),
                    Style::default().fg(DIM),
                )));
            }
        }
        Paragraph::new(lines)
    } else {
        Paragraph::new(Line::from(Span::styled(
            "No applications found.",
            Style::default().fg(DIM).italic(),
        )))
    };

    frame.render_widget(
        detail
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DIM))
                    .title(" Detail "),
            )
            .wrap(Wrap { trim: true }),
        split[1],
    );
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let (text, style) = match &app.status {
        Some(s) if s.is_live() => {
            let color = if s.is_error { ERR } else { OK };
            (s.message.clone(), Style::default().fg(color))
        }
        _ => (String::new(), Style::default()),
    };
    let p = Paragraph::new(text)
        .style(style)
        .alignment(Alignment::Left);
    frame.render_widget(p, area);
}

fn draw_help(frame: &mut Frame, area: Rect, app: &App) {
    let base = "q quit · Tab/Shift-Tab switch · ↑↓ move · r reload";
    let context = match app.tab {
        Tab::BaseDirs => "",
        Tab::MimeTypes => " · / filter · Enter set default (prompts app)",
        Tab::Applications => " · / filter",
    };
    let help = format!("{}{}", base, context);
    let p = Paragraph::new(help)
        .style(Style::default().fg(DIM))
        .alignment(Alignment::Left);
    frame.render_widget(p, area);
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        // char-boundary safe truncation
        let mut end = max;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        &s[..end]
    }
}
