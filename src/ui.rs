use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, Cell, Row, Table, Paragraph},
    Frame,
};
use crate::app::App;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.area());

    draw_header(f, chunks[0]);
    draw_process_table(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, area: Rect) {
    let text = " IdleKiller - Find and Kill Inactive Processes ".bold().cyan();
    let header = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .centered();
    f.render_widget(header, area);
}

fn draw_process_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_cells = vec![
        ("PID", crate::app::SortColumn::Pid),
        ("Name", crate::app::SortColumn::Name),
        ("Status", crate::app::SortColumn::Status),
        ("CPU (%)", crate::app::SortColumn::Cpu),
        ("Memory (MB)", crate::app::SortColumn::Memory),
        ("Search", crate::app::SortColumn::Pid),
    ]
    .into_iter()
    .map(|(name, col)| {
        let text = if name == "Search" {
            name.to_string()
        } else if app.sort_column == col {
            let indicator = match app.sort_direction {
                crate::app::SortDirection::Asc => "▲",
                crate::app::SortDirection::Desc => "▼",
            };
            format!("{} {}", name, indicator)
        } else {
            name.to_string()
        };
        Cell::from(text).style(Style::default().fg(Color::Yellow))
    });
    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .height(1)
        .bottom_margin(1);

    let rows: Vec<Row> = app.processes.iter().map(|p| {
        let is_idle = p.cpu < 0.1 && (p.status == "Sleeping" || p.status == "Idle");
        
        let style = if is_idle && p.mem_mb > 50.0 {
            Style::default().fg(Color::Magenta)
        } else if is_idle {
            Style::default().fg(Color::Blue)
        } else {
            Style::default().fg(Color::Gray)
        };

        Row::new(vec![
            Cell::from(p.pid.to_string()),
            Cell::from(p.name.clone()),
            Cell::from(p.status.clone()),
            Cell::from(format!("{:.1}%", p.cpu)),
            Cell::from(format!("{:.1} MB", p.mem_mb)),
            Cell::from("🔍 [s]"),
        ]).style(style)
    }).collect();

    let table = Table::new(rows, [
        Constraint::Length(8),
        Constraint::Min(20),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(15),
        Constraint::Length(8),
    ])
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Processes (v/j=down, ^/k=up, s=search) "))
    .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
    .column_spacing(2);

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_footer(f: &mut Frame, app: &mut App, area: Rect) {
    let message = if let Some(ref msg) = app.message {
        format!(" {} | Controls: [q] Quit, [up/down] Nav, [s] Search, [Enter/x] Kill ", msg)
    } else {
        " Controls: [q] Quit, [up/down] Nav, [s] Search, [Enter/x] Kill Selected ".to_string()
    };

    let p = Paragraph::new(message)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(p, area);
}
