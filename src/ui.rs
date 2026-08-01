use crate::app::{App, SortColumn, SortDirection};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Cell, Gauge, Paragraph, Row, Table},
};

/// Strip control characters and truncate process names for safe display.
fn clean_name(name: &str) -> String {
    // ponytail: strip control chars to prevent terminal escape injection
    let clean: String = name.chars().filter(|c| !c.is_control()).collect();
    // Truncate to 80 chars (by char count, not bytes) to prevent column overflow
    // ponytail: char count avoids panic on multi-byte boundary
    if clean.chars().count() > 80 {
        format!("{}…", clean.chars().take(79).collect::<String>())
    } else {
        clean
    }
}

pub const HEADER_AREA_HEIGHT: u16 = 3;
pub const STATS_AREA_HEIGHT: u16 = 5;
pub const TABLE_HEADER_HEIGHT: u16 = 1;
pub const TABLE_MARGIN_BOTTOM: u16 = 0;

// Column constraints
pub const COL_PID_WIDTH: u16 = 8;
pub const COL_NAME_MIN_WIDTH: u16 = 20;
pub const COL_STATUS_WIDTH: u16 = 12;
pub const COL_CPU_WIDTH: u16 = 10;
pub const COL_MEM_WIDTH: u16 = 15;

/// Must be kept in sync with draw_process_table column constraints + column_spacing.
pub fn handle_header_click(app: &mut App, col_x: u16, row_y: u16, term_width: u16) {
    // Table area starts after header (3) + stats (5); the header row is inside the top border.
    let table_top = HEADER_AREA_HEIGHT + STATS_AREA_HEIGHT; // = 8
    let header_y = table_top + 1; // inside top border of the main panel
    if row_y != header_y || col_x == 0 {
        return;
    }

    // Reconstruct column boundaries matching draw_process_table
    let spacing: u16 = 2;
    // Fixed total excludes Name which gets the remainder
    let fixed: u16 = COL_PID_WIDTH + COL_STATUS_WIDTH + COL_CPU_WIDTH + COL_MEM_WIDTH + spacing * 4;
    let name_w = (term_width.saturating_sub(2))
        .saturating_sub(fixed)
        .max(COL_NAME_MIN_WIDTH);

    let cols: [(u16, SortColumn); 5] = [
        (COL_PID_WIDTH, SortColumn::Pid),
        (name_w, SortColumn::Name),
        (COL_STATUS_WIDTH, SortColumn::Status),
        (COL_CPU_WIDTH, SortColumn::Cpu),
        (COL_MEM_WIDTH, SortColumn::Memory),
    ];

    let mut x = 1u16; // after left border of the main panel
    for (w, col) in &cols {
        if col_x >= x && col_x < x + w {
            app.toggle_sort(*col);
            return;
        }
        x += w + spacing;
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = main_block.inner(f.area());
    f.render_widget(main_block, f.area());

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
        .split(inner);

    draw_header(f, chunks[0]);
    draw_system_stats(f, app, chunks[1]);
    draw_process_table(f, app, chunks[2]);
    draw_footer(f, app, chunks[3]);
}

fn draw_header(f: &mut Frame, area: Rect) {
    let text = " IdleKiller - Find and Kill Inactive Processes ";
    let header = Paragraph::new(text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
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
        .gauge_style(Style::default().fg(if cpu_display > 80.0 {
            Color::Red
        } else if cpu_display > 50.0 {
            Color::Yellow
        } else {
            Color::Cyan
        }))
        .percent(cpu_display.clamp(0.0, 100.0) as u16)
        .label(format!("CPU: {:.1}%", cpu_display.max(0.0)));

    // RAM Usage Gauge
    let ram_display_used = stats.ram_used_mb;
    let ram_display_total = stats.ram_total_mb;
    let ram_percent = if ram_display_total > 0.0 {
        ((ram_display_used / ram_display_total * 100.0) as u16).min(100)
    } else {
        0
    };
    let ram_gauge = Gauge::default()
        .gauge_style(Style::default().fg(if ram_percent > 80 {
            Color::Red
        } else if ram_percent > 50 {
            Color::Yellow
        } else {
            Color::Cyan
        }))
        .percent(ram_percent)
        .label(format!(
            "RAM: {:.1}/{:.1}GB",
            ram_display_used / 1024.0,
            ram_display_total / 1024.0
        ));

    // Uptime Text
    let uptime_text = format!("Uptime\n{}", uptime_str);
    let uptime_paragraph = Paragraph::new(uptime_text)
        .style(Style::default().fg(Color::Gray))
        .centered();

    // Layout for the three stats
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    f.render_widget(cpu_gauge, chunks[0]);
    f.render_widget(ram_gauge, chunks[1]);
    f.render_widget(uptime_paragraph, chunks[2]);
}

fn draw_process_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_cells = vec![
        ("PID", SortColumn::Pid),
        ("Name", SortColumn::Name),
        ("Status", SortColumn::Status),
        ("CPU (%)", SortColumn::Cpu),
        ("Memory (MB)", SortColumn::Memory),
    ]
    .into_iter()
    .map(|(name, col)| {
        let text = if app.sort_column == col {
            let indicator = match app.sort_direction {
                SortDirection::Asc => "▲",
                SortDirection::Desc => "▼",
            };
            format!("{} {}", name, indicator)
        } else {
            name.to_string()
        };
        Cell::from(text).style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .height(TABLE_HEADER_HEIGHT)
        .bottom_margin(TABLE_MARGIN_BOTTOM);

    // If searching, show the search query in the header area or a special row
    // Here we'll show it in the footer area instead for a cleaner terminal look

    let rows: Vec<Row> = app
        .processes
        .iter()
        .map(|p| {
            let is_idle = p.cpu < crate::app::IDLE_CPU_THRESHOLD
                && (p.status == "Sleeping" || p.status == "Idle");

            let style = if is_idle && p.mem_mb > crate::app::WASTEFUL_MEM_MB {
                // Idle + High Memory: Warning state (Yellow)
                Style::default().fg(Color::Yellow)
            } else if is_idle {
                // Idle + Low Memory: Normal but inactive (Gray)
                Style::default().fg(Color::Gray)
            } else {
                // Active process (Cyan)
                Style::default().fg(Color::Cyan)
            };

            Row::new(vec![
                Cell::from(p.pid.to_string()),
                Cell::from(clean_name(&p.name)),
                Cell::from(p.status.clone()),
                Cell::from(format!("{:.1}%", p.cpu)),
                Cell::from(format!("{:.1} MB", p.mem_mb)),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(COL_PID_WIDTH),
            Constraint::Min(COL_NAME_MIN_WIDTH),
            Constraint::Length(COL_STATUS_WIDTH),
            Constraint::Length(COL_CPU_WIDTH),
            Constraint::Length(COL_MEM_WIDTH),
        ],
    )
    .header(header)
    .row_highlight_style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
    .column_spacing(2);

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_footer(f: &mut Frame, app: &mut App, area: Rect) {
    let message = if app.is_searching {
        format!(
            " SEARCH: [{}_] | [Esc] Cancel, [Enter] Finish ",
            app.search_query
        )
    } else if let Some(ref msg) = app.message {
        let safe_msg: String = msg.chars().filter(|c| !c.is_control()).collect();
        format!(
            " {} | [Tab/r] Sort, [s] Browser search, [f] Filter, [K] Kill Wasteful, [q] Quit ",
            safe_msg
        )
    } else {
        " [Tab/r] Sort, [s] Browser search, [f] Search, [K] Kill Wasteful, [q] Quit, [↑/↓/k/j] Nav, [Enter/x] Kill ".to_string()
    };

    let p = Paragraph::new(message)
        .style(Style::default().fg(if app.is_searching {
            Color::Yellow
        } else {
            Color::Gray
        }))
        .wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(p, area);
}
