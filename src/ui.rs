use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, Paragraph, Gauge},
    Frame,
};
use crate::app::App;
 
pub const HEADER_AREA_HEIGHT: u16 = 2;
pub const STATS_AREA_HEIGHT: u16 = 3;
pub const TABLE_HEADER_HEIGHT: u16 = 1;
pub const TABLE_MARGIN_BOTTOM: u16 = 1;

// Column constraints
pub const COL_PID_WIDTH: u16 = 8;
pub const COL_NAME_MIN_WIDTH: u16 = 20;
pub const COL_STATUS_WIDTH: u16 = 12;
pub const COL_CPU_WIDTH: u16 = 10;
pub const COL_MEM_WIDTH: u16 = 15;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(HEADER_AREA_HEIGHT),
                Constraint::Length(STATS_AREA_HEIGHT),
                Constraint::Min(10),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.area());

    draw_header(f, chunks[0]);
    draw_system_stats(f, app, chunks[1]);
    draw_process_table(f, app, chunks[2]);
    draw_footer(f, app, chunks[3]);
}

fn draw_header(f: &mut Frame, area: Rect) {
    let text = " IdleKiller - Find and Kill Inactive Processes ";
    let header = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
        .centered();
    f.render_widget(header, area);
}

fn draw_system_stats(f: &mut Frame, app: &mut App, area: Rect) {
    let stats = &app.system_stats;
    
    // Calculate uptime components
    let days = stats.uptime_seconds / 86400;
    let hours = (stats.uptime_seconds % 86400) / 3600;
    let minutes = (stats.uptime_seconds % 3600) / 60;
    let seconds = stats.uptime_seconds % 60;
    let uptime_str = if days > 0 {
        format!("{}d {:02}:{:02}:{:02}", days, hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    };

    // CPU Usage Gauge
    let cpu_display = stats.cpu_usage;
    let cpu_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" CPU USAGE "))
        .gauge_style(Style::default().fg(if cpu_display > 80.0 { Color::Red } else if cpu_display > 50.0 { Color::Yellow } else { Color::Cyan }))
        .percent((cpu_display as u16).min(100))
        .label(format!("{}%", cpu_display));

    // RAM Usage Gauge
    let ram_display_used = stats.ram_used_mb;
    let ram_display_total = stats.ram_total_mb;
    let ram_percent = if ram_display_total > 0.0 {
        ((ram_display_used / ram_display_total * 100.0) as u16).min(100)
    } else {
        0
    };
    let ram_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" RAM USAGE "))
        .gauge_style(Style::default().fg(if ram_percent > 80 { Color::Red } else if ram_percent > 50 { Color::Yellow } else { Color::Green }))
        .percent(ram_percent)
        .label(format!("{:.1}GB / {:.1}GB", ram_display_used / 1024.0, ram_display_total / 1024.0));

    // Uptime Text
    let uptime_text = format!("UPTIME\n{}\n[||||||||||||||||||..]", uptime_str);
    let uptime_paragraph = Paragraph::new(uptime_text)
        .block(Block::default().borders(Borders::ALL).title(" UPTIME "))
        .style(Style::default().fg(Color::Blue));

    // Layout for the three stats
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(34)])
        .split(area);

    f.render_widget(cpu_gauge, chunks[0]);
    f.render_widget(ram_gauge, chunks[1]);
    f.render_widget(uptime_paragraph, chunks[2]);
}

fn draw_process_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_cells = vec![
        ("PID", crate::app::SortColumn::Pid),
        ("Name", crate::app::SortColumn::Name),
        ("Status", crate::app::SortColumn::Status),
        ("CPU (%)", crate::app::SortColumn::Cpu),
        ("Memory (MB)", crate::app::SortColumn::Memory),
    ]
    .into_iter()
    .map(|(name, col)| {
        let text = if app.sort_column == col {
            let indicator = match app.sort_direction {
                crate::app::SortDirection::Asc => "▲",
                crate::app::SortDirection::Desc => "▼",
            };
            format!("{} {}", name, indicator)
        } else {
            name.to_string()
        };
        Cell::from(text).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
    });
    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .height(TABLE_HEADER_HEIGHT)
        .bottom_margin(TABLE_MARGIN_BOTTOM);

    // If searching, show the search query in the header area or a special row
    // Here we'll show it in the footer area instead for a cleaner terminal look

    let rows: Vec<Row> = app.processes.iter().map(|p| {
        let is_idle = p.cpu < 0.1 && (p.status == "Sleeping" || p.status == "Idle");
        
        let style = if is_idle && p.mem_mb > 50.0 {
            // Idle + High Memory: Warning state (Yellow)
            Style::default().fg(Color::Yellow)
        } else if is_idle {
            // Idle + Low Memory: Normal but inactive (White)
            Style::default().fg(Color::White)
        } else {
            // Active process (Cyan)
            Style::default().fg(Color::Cyan)
        };

        Row::new(vec![
            Cell::from(p.pid.to_string()),
            Cell::from(p.name.clone()),
            Cell::from(p.status.clone()),
            Cell::from(format!("{:.1}%", p.cpu)),
            Cell::from(format!("{:.1} MB", p.mem_mb)),
        ]).style(style)
    }).collect();

    let table = Table::new(rows, [
        Constraint::Length(COL_PID_WIDTH),
        Constraint::Min(COL_NAME_MIN_WIDTH),
        Constraint::Length(COL_STATUS_WIDTH),
        Constraint::Length(COL_CPU_WIDTH),
        Constraint::Length(COL_MEM_WIDTH),
    ])
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Processes (v/j=down, ^/k=up, s=search) "))
    .row_highlight_style(Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD))
    .column_spacing(2);

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_footer(f: &mut Frame, app: &mut App, area: Rect) {
    let message = if app.is_searching {
        format!(" SEARCH: [{}_] | [Esc] Cancel, [Enter] Finish ", app.search_query)
    } else if let Some(ref msg) = app.message {
        format!(" {} | [f] Search, [K] Kill Wasteful, [q] Quit, [Enter/x] Kill ", msg)
    } else {
        " [f] Search, [K] Kill Wasteful, [q] Quit, [up/down] Nav, [Enter/x] Kill ".to_string()
    };

    let p = Paragraph::new(message)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(if app.is_searching { Color::Yellow } else { Color::White }));
    f.render_widget(p, area);
}
