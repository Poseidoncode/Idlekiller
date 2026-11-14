use crate::app::{App, SortColumn, SortDirection};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Clear, LineGauge, Paragraph, Row, Table, TableState, Wrap},
};

// ─────────────────────────────────────────────────────────────────────────────
//  Color System — Commercial palette: Tech-Blue core + Silver accent layer
//  Inspired by enterprise monitoring dashboards (Datadog, Grafana dark mode).
// ─────────────────────────────────────────────────────────────────────────────

/// Core tech-blue: electric signature color
const C_TECH: Color = Color::Rgb(0, 190, 255);
/// Deep variant: denser blue for depth contrast
const C_DEEP: Color = Color::Rgb(0, 100, 200);
/// Ice highlight: sorted columns / focus state
const C_ICE: Color = Color::Rgb(160, 235, 255);
/// Phosphor-green: warm/cool contrast, terminal "active" signal
const C_PHOS: Color = Color::Rgb(0, 215, 105);
/// Amber-warning: split-complementary to tech-blue
const C_AMBER: Color = Color::Rgb(255, 180, 25);
/// Coral-danger: full complement pop, critical states only
const C_CORAL: Color = Color::Rgb(255, 65, 55);
/// Silver-accent: the commercial differentiator — warm neutral for labels
const C_SILVER: Color = Color::Rgb(185, 200, 215);
/// Steel-muted: desaturated blue-gray for secondary text
const C_STEEL: Color = Color::Rgb(80, 115, 155);
/// Slate-dim: dark blue-gray, border chrome
const C_SLATE: Color = Color::Rgb(22, 44, 80);
/// Slate-mid: slightly lighter for accent borders
const C_SLATE_MID: Color = Color::Rgb(30, 58, 100);
/// Abyss-bg: near-black with blue undertone (even rows)
const C_ABYSS: Color = Color::Rgb(4, 9, 20);
/// Surface-bg: slightly lighter panel (odd rows / header chrome)
const _C_SURFACE: Color = Color::Rgb(9, 18, 40);
/// Elevated-bg: header row background
const C_ELEVATED: Color = Color::Rgb(14, 28, 58);
/// Selected highlight: deep blue glow without washing out text
const C_SEL_BG: Color = Color::Rgb(0, 38, 88);
/// Primary text: blue-tinted off-white
const C_TEXT: Color = Color::Rgb(218, 235, 250);
/// Dim text: readable mid-tone
const C_DIM_TXT: Color = Color::Rgb(85, 120, 155);

// ─────────────────────────────────────────────────────────────────────────────
//  Layout constants
// ─────────────────────────────────────────────────────────────────────────────

pub const HEADER_AREA_HEIGHT: u16 = 3;
pub const STATS_AREA_HEIGHT: u16 = 5;
pub const TABLE_HEADER_HEIGHT: u16 = 1;
pub const TABLE_MARGIN_BOTTOM: u16 = 1;

pub const COL_PID_WIDTH: u16 = 8;
pub const COL_STATUS_WIDTH: u16 = 12;
pub const COL_CPU_WIDTH: u16 = 10;
pub const COL_MEM_WIDTH: u16 = 15;

// ─────────────────────────────────────────────────────────────────────────────
//  Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn clean_name(name: &str) -> String {
    // strip control chars to prevent terminal escape injection
    let clean: String = name.chars().filter(|c| !c.is_control()).collect();
    // char count avoids panic on multi-byte boundary
    if clean.chars().count() > 80 {
        format!("{}…", clean.chars().take(79).collect::<String>())
    } else {
        clean
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Click handler (logic unchanged; must stay in sync with column constraints)
// ─────────────────────────────────────────────────────────────────────────────

/// Width calculations are shared by the renderer and mouse hit-testing.
/// Compact mode retains PID, name, CPU and memory on terminals as narrow as 50 columns.
fn table_columns(width: u16) -> Vec<(SortColumn, u16)> {
    if width < 72 {
        let name = width.saturating_sub(2 + 7 + 8 + 12 + 3).max(1);
        vec![
            (SortColumn::Pid, 7),
            (SortColumn::Name, name),
            (SortColumn::Cpu, 8),
            (SortColumn::Memory, 12),
        ]
    } else {
        let name = width.saturating_sub(2 + COL_PID_WIDTH + COL_STATUS_WIDTH + COL_CPU_WIDTH + COL_MEM_WIDTH + 4).max(1);
        vec![
            (SortColumn::Pid, COL_PID_WIDTH),
            (SortColumn::Name, name),
            (SortColumn::Status, COL_STATUS_WIDTH),
            (SortColumn::Cpu, COL_CPU_WIDTH),
            (SortColumn::Memory, COL_MEM_WIDTH),
        ]
    }
}

pub fn handle_header_click(app: &mut App, col_x: u16, row_y: u16, term_width: u16) {
    let header_y = HEADER_AREA_HEIGHT + STATS_AREA_HEIGHT + 1;
    if row_y != header_y || col_x == 0 { return; }
    let mut x = 1u16; // table border
    for (column, width) in table_columns(term_width) {
        if col_x >= x && col_x < x.saturating_add(width) {
            app.toggle_sort(column);
            return;
        }
        x = x.saturating_add(width).saturating_add(1); // .column_spacing(1)
    }
}

/// Click-to-select respects the table's visible offset after scrolling.
pub fn handle_process_click(app: &mut App, row_y: u16, term_height: u16) {
    let table_top = HEADER_AREA_HEIGHT + STATS_AREA_HEIGHT;
    let table_height = term_height.saturating_sub(HEADER_AREA_HEIGHT + STATS_AREA_HEIGHT + 3);
    let first_row_y = table_top + 3; // top border + header + header margin
    if row_y < first_row_y || row_y >= table_top + table_height.saturating_sub(1) { return; }
    let index = app.table_state.offset() + usize::from(row_y - first_row_y);
    if index < app.processes.len() {
        app.cancel_pending_kills();
        app.table_state.select(Some(index));
        app.dirty = true;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Entry point
// ─────────────────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &mut App) {
    if f.area().width < 50 || f.area().height < 16 {
        let warning = Paragraph::new("Terminal too small — resize to at least 50 x 16")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(warning, f.area());
        return;
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HEADER_AREA_HEIGHT),
            Constraint::Length(STATS_AREA_HEIGHT),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    draw_header(f, chunks[0]);
    draw_system_stats(f, app, chunks[1]);
    draw_process_table(f, app, chunks[2]);
    draw_footer(f, app, chunks[3]);
    draw_confirmation(f, app);
    draw_action_report(f, app);
}

fn draw_action_report(f: &mut Frame, app: &App) {
    let Some(report) = app.action_report() else { return; };
    let area = f.area();
    let width = area.width.saturating_sub(4).min(72);
    let height = (report.outcomes.len() as u16 + 8).min(area.height);
    let dialog = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    let mut lines = vec![
        format!("Signal requests: {} / {}", report.requested, report.total),
        format!("Missing: {}  Changed: {}  Denied: {}  Failed: {}",
            report.missing, report.changed, report.denied, report.failed),
    ];
    for (pid, result) in &report.outcomes {
        lines.push(format!("PID {pid}: {result}"));
    }
    let skipped = report.total.saturating_sub(report.outcomes.len());
    if skipped > 0 {
        lines.push(format!("Not attempted: {skipped}"));
    }
    if report.expired { lines.push("Confirmation expired / worker timed out".to_string()); }
    lines.push(String::new());
    lines.push("Signal requested does NOT prove exit.".to_string());
    lines.push("Press any key to close this report.".to_string());
    let paragraph = Paragraph::new(lines.join("\n"))
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Thick)
            .border_style(Style::default().fg(C_AMBER)).title(" TERMINATION RESULT "))
        .wrap(Wrap { trim: false });
    f.render_widget(Clear, dialog);
    f.render_widget(paragraph, dialog);
}

/// Confirmation remains readable on narrower screens, instead of relying on a
/// single footer line that may clip the names and the actual action.
fn draw_confirmation(f: &mut Frame, app: &App) {
    if !app.has_pending_kills() { return; }
    let area = f.area();
    let width = area.width.saturating_sub(4).min(78);
    let height = (app.confirmation_target_lines().len() as u16 + 6).min(area.height);
    let dialog = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    let mut lines = vec!["Terminate these processes?".to_string()];
    lines.extend(app.confirmation_target_lines());
    lines.push(String::new());
    lines.push("Unsaved work can be lost.".to_string());
    lines.push("Same key within 8s confirms. Esc cancels.".to_string());
    let paragraph = Paragraph::new(lines.join("\n"))
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Thick)
            .border_style(Style::default().fg(C_AMBER))
            .title(" CONFIRM TERMINATION "))
        .wrap(Wrap { trim: false });
    f.render_widget(Clear, dialog);
    f.render_widget(paragraph, dialog);
}


// ─────────────────────────────────────────────────────────────────────────────
//  Header — commercial branding with version tag
// ─────────────────────────────────────────────────────────────────────────────

fn draw_header(f: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "  IDLE",
            Style::default().fg(C_TEXT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "KILLER",
            Style::default().fg(C_TECH).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ·  ", Style::default().fg(C_SLATE_MID)),
        Span::styled("Process Monitor & Cleaner", Style::default().fg(C_SILVER)),
        Span::styled("  ·  ", Style::default().fg(C_SLATE_MID)),
        Span::styled("v0.1", Style::default().fg(C_STEEL)),
        Span::styled("  ", Style::default()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::default().fg(C_DEEP)),
    )
    .alignment(Alignment::Center);

    f.render_widget(title, area);
}

// ─────────────────────────────────────────────────────────────────────────────
//  System Stats — cleaner label hierarchy with process count
// ─────────────────────────────────────────────────────────────────────────────

fn draw_system_stats(f: &mut Frame, app: &mut App, area: Rect) {
    let stats = &app.system_stats;

    // Uptime formatting
    let days = stats.uptime_seconds / 86400;
    let hours = (stats.uptime_seconds % 86400) / 3600;
    let minutes = (stats.uptime_seconds % 3600) / 60;
    let seconds = stats.uptime_seconds % 60;
    let uptime_str = if days > 0 {
        format!("{}d {:02}:{:02}:{:02}", days, hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    };

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_SLATE_MID))
        .title(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(
                "SYSTEM",
                Style::default().fg(C_SILVER).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" OVERVIEW ", Style::default().fg(C_STEEL)),
        ]));
    let inner_area = outer.inner(area);
    f.render_widget(outer, area);

    // ── CPU gauge ──────────────────────────────────────────────────────────
    let cpu_val = stats.cpu_usage;
    let cpu_ratio = (cpu_val.clamp(0.0, 100.0) / 100.0) as f64;
    let cpu_color = if cpu_val > 80.0 {
        C_CORAL
    } else if cpu_val > 50.0 {
        C_AMBER
    } else {
        C_TECH
    };

    let cpu_gauge = LineGauge::default()
        .block(Block::default().title(Line::from(vec![Span::styled(
            "CPU  ",
            Style::default().fg(C_SILVER).add_modifier(Modifier::BOLD),
        )])))
        .filled_style(Style::default().fg(cpu_color).add_modifier(Modifier::BOLD))
        .unfilled_style(Style::default().fg(C_SLATE))
        .ratio(cpu_ratio)
        .label(Line::from(vec![Span::styled(
            format!("{:5.1}%", cpu_val.max(0.0)),
            Style::default().fg(cpu_color).add_modifier(Modifier::BOLD),
        )]));

    // ── RAM gauge ──────────────────────────────────────────────────────────
    let ram_used = stats.ram_used_mb;
    let ram_total = stats.ram_total_mb;
    let ram_ratio = if ram_total > 0.0 {
        (ram_used / ram_total).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let ram_color = if ram_ratio > 0.8 {
        C_CORAL
    } else if ram_ratio > 0.6 {
        C_AMBER
    } else {
        C_PHOS
    };

    let ram_gauge = LineGauge::default()
        .block(Block::default().title(Line::from(vec![Span::styled(
            "MEM  ",
            Style::default().fg(C_SILVER).add_modifier(Modifier::BOLD),
        )])))
        .filled_style(Style::default().fg(ram_color).add_modifier(Modifier::BOLD))
        .unfilled_style(Style::default().fg(C_SLATE))
        .ratio(ram_ratio)
        .label(Line::from(vec![Span::styled(
            format!("{:.1}/{:.1}G", ram_used / 1024.0, ram_total / 1024.0),
            Style::default().fg(ram_color).add_modifier(Modifier::BOLD),
        )]));

    // ── Right panel: compact labels actually fit 50-column terminals ──────
    let proc_count = app.processes.len();
    let compact = area.width < 72;
    let uptime_label = if compact { "UP " } else { "UPTIME  " };
    let count_label = if compact { "N " } else { "PROCS   " };
    let scan_label = if compact { "S " } else { "SCAN    " };
    let right_para = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(uptime_label, Style::default().fg(C_STEEL)),
            Span::styled(
                uptime_str,
                Style::default().fg(C_ICE).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(count_label, Style::default().fg(C_STEEL)),
            Span::styled(
                if area.width >= 85 {
                    format!("{}/{}", proc_count, app.total_process_count())
                } else {
                    proc_count.to_string()
                },
                Style::default().fg(C_SILVER).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(scan_label, Style::default().fg(C_STEEL)),
            Span::styled(
                format!("{}ms", stats.scan_duration_ms),
                Style::default().fg(C_ICE),
            ),
        ]),
    ])
    .alignment(Alignment::Left);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(37),
            Constraint::Percentage(37),
            Constraint::Percentage(26),
        ])
        .split(inner_area);

    f.render_widget(cpu_gauge, chunks[0]);
    f.render_widget(ram_gauge, chunks[1]);
    f.render_widget(right_para, chunks[2]);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Process Table — premium header with Unicode sort indicators
// ─────────────────────────────────────────────────────────────────────────────

fn draw_process_table(f: &mut Frame, app: &mut App, area: Rect) {
    let columns = table_columns(area.width);
    let header_cells = columns.iter().map(|(col, _)| {
        let label = match col {
            SortColumn::Pid => "PID",
            SortColumn::Name => "NAME",
            SortColumn::Status => "STATUS",
            SortColumn::Cpu => "CPU %",
            SortColumn::Memory => if area.width < 72 { "MEM" } else { "MEM (MB)" },
        };
        let indicator = if app.sort_column == *col {
            match app.sort_direction { SortDirection::Asc => " ▲", SortDirection::Desc => " ▼" }
        } else { "" };
        let foreground = if app.sort_column == *col { C_ICE } else { C_SILVER };
        Cell::from(format!("{label}{indicator}"))
            .style(Style::default().fg(foreground).bg(C_ELEVATED).add_modifier(Modifier::BOLD))
    });
    let header = Row::new(header_cells)
        .style(Style::default().bg(C_ELEVATED))
        .height(TABLE_HEADER_HEIGHT)
        .bottom_margin(TABLE_MARGIN_BOTTOM);

    // Virtualize rows. Allocating and formatting every process on every refresh
    // wastes CPU and memory when only ~10-50 rows fit on the screen.
    let count = app.processes.len();
    let visible_rows = usize::from(area.height.saturating_sub(4)).max(1);
    let selected = app.table_state.selected().filter(|&n| n < count);
    let mut first = app.table_state.offset().min(count.saturating_sub(visible_rows));
    if let Some(index) = selected {
        if index < first { first = index; }
        if index >= first.saturating_add(visible_rows) {
            first = index + 1 - visible_rows;
        }
    }
    let end = first.saturating_add(visible_rows).min(count);
    let rows: Vec<Row> = app.processes[first..end].iter().map(|&index| {
        let p = app.process_info_from_snapshot(index);
        let candidate = p.candidate;
        let idle = p.cpu < crate::app::IDLE_CPU_THRESHOLD
            && matches!(p.status, "Sleeping" | "Idle");
        let (status_label, status_color) = match p.status {
            "Running" => ("● Run", C_PHOS),
            "Sleeping" => ("○ Sleep", C_STEEL),
            "Idle" => ("· Idle", C_DIM_TXT),
            "Zombie" => ("✖ Zombie", C_CORAL),
            "Stopped" => ("■ Stop", C_STEEL),
            _ => ("? Other", C_DIM_TXT),
        };
        let cpu_color = if p.cpu > 80.0 { C_CORAL }
            else if p.cpu > 50.0 { C_AMBER }
            else if p.cpu > 1.0 { C_TECH }
            else { C_DIM_TXT };
        let mem_color = if candidate { C_AMBER }
            else if p.mem_mb > 1000.0 { C_TECH }
            else { C_DIM_TXT };
        let name_color = if candidate { C_AMBER }
            else if !p.can_terminate || idle { C_DIM_TXT }
            else { C_TEXT };
        let prefix = if candidate { "⚠ " }
            else if !p.can_terminate { "· " }
            else { "  " };
        let cells: Vec<Cell> = columns.iter().map(|(col, _)| match col {
            SortColumn::Pid => Cell::from(format!("{:>6}", p.pid))
                .style(Style::default().fg(C_STEEL)),
            SortColumn::Name => Cell::from(format!("{}{}", prefix, clean_name(&p.name)))
                .style(Style::default().fg(name_color)),
            SortColumn::Status => Cell::from(status_label)
                .style(Style::default().fg(status_color)),
            SortColumn::Cpu => Cell::from(format!("{:>6.1}%", p.cpu))
                .style(Style::default().fg(cpu_color)),
            SortColumn::Memory => Cell::from(format!("{:>7.1}MB", p.mem_mb))
                .style(Style::default().fg(mem_color)),
        }).collect();
        Row::new(cells)
    }).collect();

    let title = Line::from(vec![
        Span::styled(" PROCESSES", Style::default().fg(C_SILVER).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  {count} tasks "), Style::default().fg(C_STEEL)),
    ]);
    let constraints: Vec<Constraint> = columns.iter()
        .map(|(_, width)| Constraint::Length(*width)).collect();
    let table = Table::new(rows, constraints)
        .header(header)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(C_SLATE_MID))
            .title(title))
        .row_highlight_style(Style::default().bg(C_SEL_BG).fg(C_ICE).add_modifier(Modifier::BOLD))
        .column_spacing(1);

    let mut viewport_state = TableState::default();
    viewport_state.select(selected.filter(|&i| i >= first && i < end).map(|i| i - first));
    f.render_stateful_widget(table, area, &mut viewport_state);
    *app.table_state.offset_mut() = first + viewport_state.offset();
}

// ─────────────────────────────────────────────────────────────────────────────
//  Footer — grouped keybind hints with section separators
// ─────────────────────────────────────────────────────────────────────────────

fn draw_footer(f: &mut Frame, app: &mut App, area: Rect) {
    let (content, border_color) = if app.is_searching {
        let safe_q: String = app
            .search_query
            .chars()
            .filter(|c| !c.is_control())
            .collect();
        (
            Line::from(vec![
                Span::styled(
                    " ⌕ FILTER ",
                    Style::default()
                        .fg(C_ABYSS)
                        .bg(C_AMBER)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{}_", safe_q),
                    Style::default().fg(C_TEXT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "   Esc Cancel   Enter Confirm",
                    Style::default().fg(C_STEEL),
                ),
            ]),
            C_AMBER,
        )
    } else if app.worker_unavailable() || app.snapshot_delayed() {
        let warning = if app.worker_unavailable() {
            "  Process worker stopped — restart required"
        } else {
            "  Process collector delayed — actions disabled"
        };
        (
            Line::from(vec![
                Span::styled(" ! ", Style::default().fg(C_ABYSS).bg(C_AMBER).add_modifier(Modifier::BOLD)),
                Span::styled(warning, Style::default().fg(C_TEXT)),
            ]),
            C_AMBER,
        )
    } else if let Some(ref msg) = app.message {
        let safe_msg: String = msg.chars().filter(|c| !c.is_control()).collect();
        (
            Line::from(vec![
                // Neutral information icon: a refusal or error is not a success.
                Span::styled(
                    " i ",
                    Style::default()
                        .fg(C_ABYSS)
                        .bg(C_STEEL)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(safe_msg, Style::default().fg(C_TEXT)),
            ]),
            C_STEEL,
        )
    } else {
        (
            Line::from(vec![
                // Navigate group
                kbd("↑↓"),
                act("Nav"),
                vsep(),
                kbd("Enter"),
                act("Confirm Kill"),
                vsep(),
                kbd("K"),
                act("Review Cleanup"),
                // Search group
                Span::styled("  │  ", Style::default().fg(C_SLATE_MID)),
                kbd("f"),
                act("Filter"),
                vsep(),
                kbd("Tab"),
                act("Sort"),
                // Quit
                Span::styled("  │  ", Style::default().fg(C_SLATE_MID)),
                kbd("q"),
                act("Quit"),
                Span::styled(" ", Style::default()),
            ]),
            C_SLATE,
        )
    };

    let footer = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(footer, area);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Footer micro-components
// ─────────────────────────────────────────────────────────────────────────────

#[inline]
fn kbd(key: &'static str) -> Span<'static> {
    // Key label: bright silver-white so it pops against the dark background
    Span::styled(
        format!(" {} ", key),
        Style::default()
            .fg(C_TEXT)
            .bg(C_ELEVATED)
            .add_modifier(Modifier::BOLD),
    )
}

#[inline]
fn act(label: &'static str) -> Span<'static> {
    // Action label: steel — recedes behind the key
    Span::styled(format!(" {} ", label), Style::default().fg(C_STEEL))
}

#[inline]
fn vsep() -> Span<'static> {
    Span::styled(" · ", Style::default().fg(C_SLATE_MID))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_and_full_columns_fit_the_terminal() {
        for width in [50, 60, 71, 72, 80, 120] {
            let columns = table_columns(width);
            let consumed = columns.iter().map(|(_, w)| usize::from(*w)).sum::<usize>()
                + columns.len() - 1 + 2;
            assert_eq!(consumed, usize::from(width), "width {width}");
            assert!(columns.iter().any(|(c, _)| *c == SortColumn::Name));
        }
    }

    #[test]
    fn interface_renders_at_minimum_and_wide_terminal_sizes() {
        for (width, height) in [(50, 16), (71, 16), (72, 16), (120, 40)] {
            let backend = ratatui::backend::TestBackend::new(width, height);
            let mut terminal = ratatui::Terminal::new(backend).expect("test terminal");
            let mut app = App::new_for_test();
            terminal.draw(|frame| draw(frame, &mut app)).expect("render without panic");
        }
    }
}
