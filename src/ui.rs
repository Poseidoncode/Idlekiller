use crate::app::{App, SortColumn, SortDirection};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, LineGauge, Paragraph, Row, Table, Wrap},
};

// ─────────────────────────────────────────────────────────────────────────────
//  Color System — Derived from core Tech-Blue RGB(0,200,255)
//  All values hand-picked via color theory; no template/Tailwind colors used.
// ─────────────────────────────────────────────────────────────────────────────

/// Core tech-blue: electric, CRT-screen feel
const C_TECH:     Color = Color::Rgb(0,   200, 255);
/// Deep variant: denser blue for depth contrast
const C_DEEP:     Color = Color::Rgb(0,   110, 210);
/// Ice highlight: lighter than core, used for sorted columns / focus
const C_ICE:      Color = Color::Rgb(140, 230, 255);
/// Phosphor-green: warm/cool contrast with the blue; terminal "active" signal
const C_PHOS:     Color = Color::Rgb(0,   225, 110);
/// Amber-warning: split-complementary to tech-blue (~30° from orange)
const C_AMBER:    Color = Color::Rgb(255, 185,  20);
/// Coral-danger: full complement pop, only for critical states
const C_CORAL:    Color = Color::Rgb(255,  70,  50);
/// Steel-muted: desaturated blue-gray for secondary text
const C_STEEL:    Color = Color::Rgb( 85, 120, 160);
/// Slate-dim: very dark blue, border chrome
const C_SLATE:    Color = Color::Rgb( 18,  38,  70);
/// Abyss-bg: near-black with blue undertone (even rows)
const C_ABYSS:    Color = Color::Rgb(  5,  10,  22);
/// Surface-bg: slightly lighter panel (odd rows / header chrome)
const C_SURFACE:  Color = Color::Rgb( 10,  20,  42);
/// Selected highlight: deep blue glow without washing out text
const C_SEL_BG:   Color = Color::Rgb(  0,  40,  90);
/// Primary text: blue-tinted off-white so it harmonises with the palette
const C_TEXT:     Color = Color::Rgb(215, 235, 248);
/// Dim text: readable mid-tone that won't compete with highlights
const C_DIM_TXT:  Color = Color::Rgb( 95, 130, 165);

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
    // ponytail: strip control chars to prevent terminal escape injection
    let clean: String = name.chars().filter(|c| !c.is_control()).collect();
    // ponytail: char count avoids panic on multi-byte boundary
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
        (COL_PID_WIDTH,   SortColumn::Pid),
        (name_w,          SortColumn::Name),
        (COL_STATUS_WIDTH,SortColumn::Status),
        (COL_CPU_WIDTH,   SortColumn::Cpu),
        (COL_MEM_WIDTH,   SortColumn::Memory),
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
//  Header
// ─────────────────────────────────────────────────────────────────────────────

fn draw_header(f: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "[ ",
            Style::default().fg(C_SLATE).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "IDLE",
            Style::default()
                .fg(C_TECH)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "KILLER",
            Style::default()
                .fg(C_DEEP)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " ]",
            Style::default().fg(C_SLATE).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  //  Process Monitor & Cleaner",
            Style::default().fg(C_STEEL),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(C_DEEP)),
    )
    .centered();

    f.render_widget(title, area);
}

// ─────────────────────────────────────────────────────────────────────────────
//  System Stats
// ─────────────────────────────────────────────────────────────────────────────

fn draw_system_stats(f: &mut Frame, app: &mut App, area: Rect) {
    let stats = &app.system_stats;

    // Uptime
    let days    = stats.uptime_seconds / 86400;
    let hours   = (stats.uptime_seconds % 86400) / 3600;
    let minutes = (stats.uptime_seconds % 3600) / 60;
    let seconds = stats.uptime_seconds % 60;
    let uptime_str = if days > 0 {
        format!("{}d {:02}h {:02}m {:02}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("{:02}h {:02}m {:02}s", hours, minutes, seconds)
    } else {
        format!("{:02}m {:02}s", minutes, seconds)
    };

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_SLATE))
        .title(Line::from(vec![
            Span::styled(
                " System Overview ",
                Style::default()
                    .fg(C_TECH)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    let inner_area = outer.inner(area);
    f.render_widget(outer, area);

    // ── CPU gauge ──────────────────────────────────────────────────────────
    let cpu_val   = stats.cpu_usage;
    let cpu_ratio = (cpu_val.clamp(0.0, 100.0) / 100.0) as f64;
    // Danger at >80 → coral (split-complement), caution at >50 → amber, else tech-blue
    let cpu_color = if cpu_val > 80.0 { C_CORAL } else if cpu_val > 50.0 { C_AMBER } else { C_TECH };

    let cpu_gauge = LineGauge::default()
        .block(Block::default().title(Line::from(vec![
            Span::styled(" CPU ", Style::default().fg(C_STEEL)),
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
    // Same colour logic: >80% coral, >60% amber, else phosphor-green (contrast with blue)
    let ram_color = if ram_ratio > 0.8 { C_CORAL } else if ram_ratio > 0.6 { C_AMBER } else { C_PHOS };

    let ram_gauge = LineGauge::default()
        .block(Block::default().title(Line::from(vec![
            Span::styled(" MEM ", Style::default().fg(C_STEEL)),
        ])))
        .filled_style(Style::default().fg(ram_color).add_modifier(Modifier::BOLD))
        .unfilled_style(Style::default().fg(C_SLATE))
        .ratio(ram_ratio)
        .label(Line::from(vec![Span::styled(
            format!("{:.1}/{:.1}G", ram_used / 1024.0, ram_total / 1024.0),
            Style::default().fg(ram_color).add_modifier(Modifier::BOLD),
        )]));

    // ── Uptime ─────────────────────────────────────────────────────────────
    let uptime_para = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "UPTIME",
            Style::default().fg(C_STEEL).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            uptime_str,
            Style::default().fg(C_ICE).add_modifier(Modifier::BOLD),
        )]),
    ])
    .centered();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(38),
            Constraint::Percentage(38),
            Constraint::Percentage(24),
        ])
        .split(inner_area);

    f.render_widget(cpu_gauge,   chunks[0]);
    f.render_widget(ram_gauge,   chunks[1]);
    f.render_widget(uptime_para, chunks[2]);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Process Table
// ─────────────────────────────────────────────────────────────────────────────

fn draw_process_table(f: &mut Frame, app: &mut App, area: Rect) {
    // ── Column headers ─────────────────────────────────────────────────────
    let col_defs: [(&str, SortColumn); 5] = [
        ("PID",       SortColumn::Pid),
        ("Name",      SortColumn::Name),
        ("Status",    SortColumn::Status),
        ("CPU %",     SortColumn::Cpu),
        ("Mem (MB)",  SortColumn::Memory),
    ];

    let header_cells = col_defs.iter().map(|(label, col)| {
        let is_sorted = app.sort_column == *col;
        let indicator = if is_sorted {
            match app.sort_direction {
                SortDirection::Asc  => " ^",
                SortDirection::Desc => " v",
            }
        } else {
            ""
        };
        // Sorted column: ice-highlight. Others: steel so they recede
        let style = if is_sorted {
            Style::default()
                .fg(C_ICE)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(C_STEEL).add_modifier(Modifier::BOLD)
        };
        Cell::from(format!("{}{}", label, indicator)).style(style)
    });

    let header = Row::new(header_cells)
        .style(Style::default().bg(C_SURFACE))
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

            // Subtle zebra using our two base shades
            let row_bg = if i % 2 == 0 { C_ABYSS } else { C_SURFACE };

            // Status text + color
            let (status_label, status_color) = match p.status.as_str() {
                "Running"            => ("> Run  ", C_PHOS),
                "Sleeping" | "Sleep" => ("~ Sleep", C_STEEL),
                "Idle"               => ("- Idle ", C_SLATE),
                "Zombie"   | "Z"     => ("X Zmb  ", C_CORAL),
                other                => {
                    let s = &other[..other.len().min(5)];
                    // We return a static-like string; format in the cell
                    let _ = s; // will handle below
                    ("? ???  ", C_DIM_TXT)
                }
            };
            // For unknown statuses, truncate properly
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

            // Memory: wasteful → amber; high → deep-blue; normal → dim
            let mem_color = if is_wasteful {
                C_AMBER
            } else if p.mem_mb > 1000.0 {
                C_TECH
            } else {
                C_DIM_TXT
            };

            // Name: active=text-white, idle=dim, wasteful=amber
            let name_prefix = if is_wasteful { "! " } else if !is_idle { "> " } else { "  " };
            let name_color  = if is_wasteful {
                C_AMBER
            } else if !is_idle {
                C_TEXT
            } else {
                C_DIM_TXT
            };

            Row::new(vec![
                Cell::from(format!("{:>6}", p.pid))
                    .style(Style::default().fg(C_DIM_TXT)),
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
            .style(Style::default().bg(row_bg))
        })
        .collect();

    let proc_count = app.processes.len();
    let title = Line::from(vec![
        Span::styled(
            " Processes ",
            Style::default().fg(C_TECH).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("[{}] ", proc_count),
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
            .border_style(Style::default().fg(C_SLATE))
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
//  Footer
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
                Span::styled(" SEARCH // ", Style::default().fg(C_AMBER).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{}_", safe_q),
                    Style::default().fg(C_TEXT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "   [Esc] Cancel   [Enter] Confirm",
                    Style::default().fg(C_STEEL),
                ),
            ]),
            C_AMBER,
        )
    } else if let Some(ref msg) = app.message {
        let safe_msg: String = msg.chars().filter(|c| !c.is_control()).collect();
        (
            Line::from(vec![
                Span::styled(" // ", Style::default().fg(C_PHOS)),
                Span::styled(safe_msg, Style::default().fg(C_TEXT)),
                Span::styled(
                    "   [Tab/r] Sort  [f] Search  [K] Kill Wasteful  [q] Quit",
                    Style::default().fg(C_STEEL),
                ),
            ]),
            C_PHOS,
        )
    } else {
        (
            Line::from(vec![
                hint("kj/Up/Dn", "Nav"),
                sep(),
                hint("Enter/x", "Kill"),
                sep(),
                hint("K", "Kill Wasteful"),
                sep(),
                hint("f", "Search"),
                sep(),
                hint("Tab/r", "Sort"),
                sep(),
                hint("q", "Quit"),
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
fn hint(key: &'static str, action: &'static str) -> Span<'static> {
    // Key in tech-blue, action in dim — creates clear visual scan path
    Span::styled(
        format!(" [{}] {} ", key, action),
        Style::default().fg(C_TECH),
    )
}

#[inline]
fn sep() -> Span<'static> {
    Span::styled("|", Style::default().fg(C_SLATE))
}
