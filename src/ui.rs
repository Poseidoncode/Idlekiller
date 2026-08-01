use crate::app::{App, SortColumn, SortDirection};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, LineGauge, Paragraph, Row, Table, Wrap},
};

// ─────────────────────────────────────────────────────────────────────────────
//  Color System — Commercial palette: Tech-Blue core + Silver accent layer
//  Inspired by enterprise monitoring dashboards (Datadog, Grafana dark mode).
// ─────────────────────────────────────────────────────────────────────────────

/// Core tech-blue: electric signature color
const C_TECH:     Color = Color::Rgb(  0, 190, 255);
/// Deep variant: denser blue for depth contrast
const C_DEEP:     Color = Color::Rgb(  0, 100, 200);
/// Ice highlight: sorted columns / focus state
const C_ICE:      Color = Color::Rgb(160, 235, 255);
/// Phosphor-green: warm/cool contrast, terminal "active" signal
const C_PHOS:     Color = Color::Rgb(  0, 215, 105);
/// Amber-warning: split-complementary to tech-blue
const C_AMBER:    Color = Color::Rgb(255, 180,  25);
/// Coral-danger: full complement pop, critical states only
const C_CORAL:    Color = Color::Rgb(255,  65,  55);
/// Silver-accent: the commercial differentiator — warm neutral for labels
const C_SILVER:   Color = Color::Rgb(185, 200, 215);
/// Steel-muted: desaturated blue-gray for secondary text
const C_STEEL:    Color = Color::Rgb( 80, 115, 155);
/// Slate-dim: dark blue-gray, border chrome
const C_SLATE:    Color = Color::Rgb( 22,  44,  80);
/// Slate-mid: slightly lighter for accent borders
const C_SLATE_MID:Color = Color::Rgb( 30,  58, 100);
/// Abyss-bg: near-black with blue undertone (even rows)
const C_ABYSS:    Color = Color::Rgb(  4,   9,  20);
/// Surface-bg: slightly lighter panel (odd rows / header chrome)
const C_SURFACE:  Color = Color::Rgb(  9,  18,  40);
/// Elevated-bg: header row background
const C_ELEVATED: Color = Color::Rgb( 14,  28,  58);
/// Selected highlight: deep blue glow without washing out text
const C_SEL_BG:   Color = Color::Rgb(  0,  38,  88);
/// Primary text: blue-tinted off-white
const C_TEXT:     Color = Color::Rgb(218, 235, 250);
/// Dim text: readable mid-tone
const C_DIM_TXT:  Color = Color::Rgb( 85, 120, 155);

// ─────────────────────────────────────────────────────────────────────────────
//  Layout constants
// ─────────────────────────────────────────────────────────────────────────────

pub const HEADER_AREA_HEIGHT:  u16 = 3;
pub const STATS_AREA_HEIGHT:   u16 = 5;
pub const TABLE_HEADER_HEIGHT: u16 = 1;
pub const TABLE_MARGIN_BOTTOM: u16 = 1;

pub const COL_PID_WIDTH:      u16 = 8;
pub const COL_NAME_MIN_WIDTH: u16 = 20;
pub const COL_STATUS_WIDTH:   u16 = 12;
pub const COL_CPU_WIDTH:      u16 = 10;
pub const COL_MEM_WIDTH:      u16 = 15;

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

pub fn handle_header_click(app: &mut App, col_x: u16, row_y: u16, term_width: u16) {
    let table_top = HEADER_AREA_HEIGHT + STATS_AREA_HEIGHT;
    let header_y  = table_top + 1;
    if row_y != header_y || col_x == 0 {
        return;
    }

    let spacing: u16 = 2;
    let fixed: u16 =
        COL_PID_WIDTH + COL_STATUS_WIDTH + COL_CPU_WIDTH + COL_MEM_WIDTH + spacing * 4;
    let name_w = (term_width.saturating_sub(2))
        .saturating_sub(fixed)
        .max(COL_NAME_MIN_WIDTH);

    let cols: [(u16, SortColumn); 5] = [
        (COL_PID_WIDTH,    SortColumn::Pid),
        (name_w,           SortColumn::Name),
        (COL_STATUS_WIDTH, SortColumn::Status),
        (COL_CPU_WIDTH,    SortColumn::Cpu),
        (COL_MEM_WIDTH,    SortColumn::Memory),
    ];

    let mut x = 1u16;
    for (w, col) in &cols {
        if col_x >= x && col_x < x + w {
            app.toggle_sort(*col);
            return;
        }
        x += w + spacing;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Entry point
// ─────────────────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HEADER_AREA_HEIGHT),
            Constraint::Length(STATS_AREA_HEIGHT),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    draw_header(f, chunks[0]);
    draw_system_stats(f, app, chunks[1]);
    draw_process_table(f, app, chunks[2]);
    draw_footer(f, app, chunks[3]);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Header — commercial branding with version tag
// ─────────────────────────────────────────────────────────────────────────────

fn draw_header(f: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "  IDLE",
            Style::default()
                .fg(C_TEXT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "KILLER",
            Style::default()
                .fg(C_TECH)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  ·  ",
            Style::default().fg(C_SLATE_MID),
        ),
        Span::styled(
            "Process Monitor & Cleaner",
            Style::default().fg(C_SILVER),
        ),
        Span::styled(
            "  ·  ",
            Style::default().fg(C_SLATE_MID),
        ),
        Span::styled(
            "v0.1",
            Style::default().fg(C_STEEL),
        ),
        Span::styled(
            "  ",
            Style::default(),
        ),
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
    let days    = stats.uptime_seconds / 86400;
    let hours   = (stats.uptime_seconds % 86400) / 3600;
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
                Style::default()
                    .fg(C_SILVER)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " OVERVIEW ",
                Style::default().fg(C_STEEL),
            ),
        ]));
    let inner_area = outer.inner(area);
    f.render_widget(outer, area);

    // ── CPU gauge ──────────────────────────────────────────────────────────
    let cpu_val   = stats.cpu_usage;
    let cpu_ratio = (cpu_val.clamp(0.0, 100.0) / 100.0) as f64;
    let cpu_color = if cpu_val > 80.0 { C_CORAL } else if cpu_val > 50.0 { C_AMBER } else { C_TECH };

    let cpu_gauge = LineGauge::default()
        .block(Block::default().title(Line::from(vec![
            Span::styled("CPU  ", Style::default().fg(C_SILVER).add_modifier(Modifier::BOLD)),
        ])))
        .filled_style(Style::default().fg(cpu_color).add_modifier(Modifier::BOLD))
        .unfilled_style(Style::default().fg(C_SLATE))
        .ratio(cpu_ratio)
        .label(Line::from(vec![Span::styled(
            format!("{:5.1}%", cpu_val.max(0.0)),
            Style::default().fg(cpu_color).add_modifier(Modifier::BOLD),
        )]));

    // ── RAM gauge ──────────────────────────────────────────────────────────
    let ram_used  = stats.ram_used_mb;
    let ram_total = stats.ram_total_mb;
    let ram_ratio = if ram_total > 0.0 {
        (ram_used / ram_total).clamp(0.0, 1.0) as f64
    } else {
        0.0
    };
    let ram_color = if ram_ratio > 0.8 { C_CORAL } else if ram_ratio > 0.6 { C_AMBER } else { C_PHOS };

    let ram_gauge = LineGauge::default()
        .block(Block::default().title(Line::from(vec![
            Span::styled("MEM  ", Style::default().fg(C_SILVER).add_modifier(Modifier::BOLD)),
        ])))
        .filled_style(Style::default().fg(ram_color).add_modifier(Modifier::BOLD))
        .unfilled_style(Style::default().fg(C_SLATE))
        .ratio(ram_ratio)
        .label(Line::from(vec![Span::styled(
            format!("{:.1}/{:.1}G", ram_used / 1024.0, ram_total / 1024.0),
            Style::default().fg(ram_color).add_modifier(Modifier::BOLD),
        )]));

    // ── Right panel: uptime + process count ────────────────────────────────
    let proc_count = app.processes.len();
    let right_para = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("UPTIME  ", Style::default().fg(C_STEEL)),
            Span::styled(
                uptime_str,
                Style::default().fg(C_ICE).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("PROCS   ", Style::default().fg(C_STEEL)),
            Span::styled(
                format!("{}", proc_count),
                Style::default().fg(C_SILVER).add_modifier(Modifier::BOLD),
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

    f.render_widget(cpu_gauge,  chunks[0]);
    f.render_widget(ram_gauge,  chunks[1]);
    f.render_widget(right_para, chunks[2]);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Process Table — premium header with Unicode sort indicators
// ─────────────────────────────────────────────────────────────────────────────

fn draw_process_table(f: &mut Frame, app: &mut App, area: Rect) {
    // ── Column headers ─────────────────────────────────────────────────────
    let col_defs: [(&str, SortColumn); 5] = [
        ("PID",      SortColumn::Pid),
        ("NAME",     SortColumn::Name),
        ("STATUS",   SortColumn::Status),
        ("CPU %",    SortColumn::Cpu),
        ("MEM (MB)", SortColumn::Memory),
    ];

    let header_cells = col_defs.iter().map(|(label, col)| {
        let is_sorted = app.sort_column == *col;
        // Unicode arrows replace ASCII ^ v — sharper visual language
        let indicator = if is_sorted {
            match app.sort_direction {
                SortDirection::Asc  => " ▲",
                SortDirection::Desc => " ▼",
            }
        } else {
            "  "
        };
        let style = if is_sorted {
            Style::default()
                .fg(C_ICE)
                .bg(C_ELEVATED)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(C_SILVER)
                .bg(C_ELEVATED)
                .add_modifier(Modifier::BOLD)
        };
        Cell::from(format!("{}{}", label, indicator)).style(style)
    });

    let header = Row::new(header_cells)
        .style(Style::default().bg(C_ELEVATED))
        .height(TABLE_HEADER_HEIGHT)
        .bottom_margin(TABLE_MARGIN_BOTTOM);

    // ── Rows ───────────────────────────────────────────────────────────────
    let rows: Vec<Row> = app
        .processes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let is_idle = p.cpu < crate::app::IDLE_CPU_THRESHOLD
                && (p.status == "Sleeping" || p.status == "Idle");
            let is_wasteful = is_idle && p.mem_mb > crate::app::WASTEFUL_MEM_MB;



            // Status: minimal indicator + concise label
            let (status_label, status_color) = match p.status.as_str() {
                "Running"            => ("● Run   ", C_PHOS),
                "Sleeping" | "Sleep" => ("○ Sleep ", C_STEEL),
                "Idle"               => ("· Idle  ", C_DIM_TXT),
                "Zombie"   | "Z"     => ("✖ Zombi ", C_CORAL),
                _                    => ("? Other ", C_DIM_TXT),
            };
            let status_display = if matches!(
                p.status.as_str(),
                "Running" | "Sleeping" | "Sleep" | "Idle" | "Zombie" | "Z"
            ) {
                status_label.to_string()
            } else {
                format!("? {:<5}", &p.status[..p.status.len().min(5)])
            };

            // CPU colour: active=tech-blue, heavy=amber, danger=coral, idle=dim
            let cpu_color = if p.cpu > 80.0 {
                C_CORAL
            } else if p.cpu > 50.0 {
                C_AMBER
            } else if p.cpu > 1.0 {
                C_TECH
            } else {
                C_DIM_TXT
            };

            // Memory: wasteful → amber; high → tech-blue; normal → dim
            let mem_color = if is_wasteful {
                C_AMBER
            } else if p.mem_mb > 1000.0 {
                C_TECH
            } else {
                C_DIM_TXT
            };

            // Name: active=primary text, idle=dim, wasteful=amber with warning marker
            let name_prefix = if is_wasteful {
                "⚠ "
            } else if !is_idle {
                "  "
            } else {
                "  "
            };
            let name_color = if is_wasteful {
                C_AMBER
            } else if !is_idle {
                C_TEXT
            } else {
                C_DIM_TXT
            };

            Row::new(vec![
                Cell::from(format!("{:>6}", p.pid))
                    .style(Style::default().fg(C_STEEL)),
                Cell::from(format!("{}{}", name_prefix, clean_name(&p.name)))
                    .style(Style::default().fg(name_color)),
                Cell::from(status_display)
                    .style(Style::default().fg(status_color)),
                Cell::from(format!("{:>7.1}%", p.cpu))
                    .style(Style::default().fg(cpu_color).add_modifier(
                        if p.cpu > 50.0 { Modifier::BOLD } else { Modifier::empty() },
                    )),
                Cell::from(format!("{:>9.1} MB", p.mem_mb))
                    .style(Style::default().fg(mem_color).add_modifier(
                        if is_wasteful { Modifier::BOLD } else { Modifier::empty() },
                    )),
            ])
            .style(Style::default())
        })
        .collect();

    let proc_count = app.processes.len();
    let title = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(
            "PROCESSES",
            Style::default().fg(C_SILVER).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {} tasks ", proc_count),
            Style::default().fg(C_STEEL),
        ),
    ]);

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
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(C_SLATE_MID))
            .title(title),
    )
    .row_highlight_style(
        Style::default()
            .bg(C_SEL_BG)
            .fg(C_ICE)
            .add_modifier(Modifier::BOLD),
    )
    .column_spacing(1);

    f.render_stateful_widget(table, area, &mut app.table_state);
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
                Span::styled(" ⌕ FILTER ", Style::default().fg(C_ABYSS).bg(C_AMBER).add_modifier(Modifier::BOLD)),
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
    } else if let Some(ref msg) = app.message {
        let safe_msg: String = msg.chars().filter(|c| !c.is_control()).collect();
        (
            Line::from(vec![
                Span::styled(" ✔ ", Style::default().fg(C_ABYSS).bg(C_PHOS).add_modifier(Modifier::BOLD)),
                Span::styled("  ", Style::default()),
                Span::styled(safe_msg, Style::default().fg(C_TEXT)),
            ]),
            C_PHOS,
        )
    } else {
        (
            Line::from(vec![
                // Navigate group
                kbd("↑↓"),
                act("Nav"),
                vsep(),
                kbd("Enter"),
                act("Kill"),
                vsep(),
                kbd("K"),
                act("Kill Wasteful"),
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
    Span::styled(
        format!(" {} ", label),
        Style::default().fg(C_STEEL),
    )
}

#[inline]
fn vsep() -> Span<'static> {
    Span::styled(" · ", Style::default().fg(C_SLATE_MID))
}
