use std::cmp::Ordering;
use std::time::{Duration, Instant, SystemTime};
use sysinfo::{Pid, ProcessStatus};

// ponytail: named thresholds instead of raw magic numbers
pub(crate) const IDLE_CPU_THRESHOLD: f32 = 0.1;
pub(crate) const WASTEFUL_MEM_MB: f64 = 500.0;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SortColumn {
    Pid,
    Name,
    Status,
    Cpu,
    Memory,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: Pid,
    pub name: String,
    pub name_lower: String,
    pub cpu: f32,
    pub mem_mb: f64,
    pub status: String,
}

#[derive(Debug)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub ram_used_mb: f64,
    pub ram_total_mb: f64,
    pub uptime_seconds: u64,
}

use ratatui::widgets::TableState;

pub struct App {
    sys: sysinfo::System,
    pub processes: Vec<ProcessInfo>,
    all_processes: Vec<ProcessInfo>,
    pub table_state: TableState,
    pub should_quit: bool,
    pub message: Option<String>,
    pub sort_column: SortColumn,
    pub sort_direction: SortDirection,
    pub system_stats: SystemStats,
    pub boot_time: SystemTime,
    pub is_searching: bool,
    pub search_query: String,
    pub dirty: bool,
    pub message_instant: Option<Instant>,
    pub confirming_kill_all: bool,
    last_kill_instant: Option<Instant>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let sys = sysinfo::System::new_all();
        
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        // Get boot time for uptime calculation
        let boot_secs = sysinfo::System::boot_time();
        let boot_time = if boot_secs > 0 {
            SystemTime::UNIX_EPOCH + Duration::from_secs(boot_secs)
        } else {
            // ponytail: fail-safe — fall back to now instead of 1970-era nonsense
            SystemTime::now()
        };
        
        let mut app = Self {
            sys,
            processes: Vec::new(),
            all_processes: Vec::new(),
            table_state,
            should_quit: false,
            message: None,
            sort_column: SortColumn::Memory,
            sort_direction: SortDirection::Desc,
            system_stats: SystemStats {
                cpu_usage: 0.0,
                ram_used_mb: 0.0,
                ram_total_mb: 0.0,
                uptime_seconds: 0,
            },
            boot_time,
            dirty: true,
            message_instant: None,
            confirming_kill_all: false,
            last_kill_instant: None,
            is_searching: false,
            search_query: String::new(),
        };
        
        // populate initial data immediately
        // ponytail: skip the CPU-delta sleep — first tick will compute real values
        app.refresh();
        app
    }

    /// Fetch process data from OS (heavy I/O)
    fn refresh_data(&mut self) {
        self.sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            // ponytail: only cpu+mem, skip disk/user/tasks/env etc
            sysinfo::ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .without_tasks()
        );
        self.update_system_stats();

        self.all_processes = self.sys.processes().iter().map(|(pid, process)| {
            ProcessInfo {
                pid: *pid,
                name: process.name().to_string_lossy().to_string(),
                name_lower: process.name().to_string_lossy().to_lowercase(),
                cpu: process.cpu_usage(),
                mem_mb: process.memory() as f64 / 1024.0 / 1024.0,
                status: match process.status() {
                    ProcessStatus::Run => "Running",
                    ProcessStatus::Sleep => "Sleeping",
                    ProcessStatus::Stop => "Stopped",
                    ProcessStatus::Zombie => "Zombie",
                    ProcessStatus::Idle => "Idle",
                    // ponytail: Parked (macOS halted-at-clean-point) mapped as Idle
                    ProcessStatus::Parked => "Idle",
                    _ => "Other",
                }.to_string(),
            }
        }).collect();
    }

    /// Filter + sort from all_processes (in-memory, no I/O)
    pub fn apply_filter(&mut self) {
        self.dirty = true;
        let search_pattern = self.search_query.to_lowercase();

        // Save selected PID before filtering/sorting so selection follows the process
        let selected_pid = self.table_state.selected()
            .and_then(|i| self.processes.get(i))
            .map(|p| p.pid);

        self.processes = self.all_processes.iter().filter(|p| {
            search_pattern.is_empty() || p.name_lower.contains(&search_pattern)
        }).cloned().collect();

        self.processes.sort_by(|a, b| {
            let ordering = match self.sort_column {
                SortColumn::Pid => a.pid.as_u32().cmp(&b.pid.as_u32()),
                SortColumn::Name => a.name_lower.cmp(&b.name_lower),
                SortColumn::Status => a.status.cmp(&b.status),
                SortColumn::Cpu => a.cpu.partial_cmp(&b.cpu).unwrap_or(Ordering::Equal),
                SortColumn::Memory => a.mem_mb.partial_cmp(&b.mem_mb).unwrap_or(Ordering::Equal),
            };
            let final_ordering = if ordering == Ordering::Equal {
                a.pid.as_u32().cmp(&b.pid.as_u32())
            } else {
                ordering
            };
            match self.sort_direction {
                SortDirection::Asc => final_ordering,
                SortDirection::Desc => final_ordering.reverse(),
            }
        });

        // Restore selection by PID so the same process stays highlighted after sort
        let new_selected = selected_pid
            .and_then(|pid| self.processes.iter().position(|p| p.pid == pid))
            .or(if self.processes.is_empty() { None } else { Some(0) });
        self.table_state.select(new_selected);
    }

    pub fn refresh(&mut self) {
        // Keep messages visible for at least 4 seconds
        let expired = self.message_instant
            .map(|t| t.elapsed() >= Duration::from_secs(4))
            .unwrap_or(true);
        if expired {
            self.message = None;
            self.message_instant = None;
        }
        self.refresh_data();
        self.apply_filter();
    }

    fn update_system_stats(&mut self) {
        // Refresh CPU and memory info
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        let cpu_usage = self.sys.global_cpu_usage();
        let ram_total = self.sys.total_memory() as f64 / 1024.0 / 1024.0;
        let ram_used = self.sys.used_memory() as f64 / 1024.0 / 1024.0;

        // Calculate uptime from boot time
        let now = SystemTime::now();
        let duration = now.duration_since(self.boot_time).unwrap_or(Duration::ZERO);
        let uptime_seconds = duration.as_secs();

        self.system_stats = SystemStats {
            cpu_usage,
            ram_used_mb: ram_used,
            ram_total_mb: ram_total,
            uptime_seconds,
        };
    }

    pub fn next(&mut self) {
        if self.processes.is_empty() {
            return;
        }
        self.dirty = true;
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.processes.len().saturating_sub(1) {
                    i // stop at last
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.processes.is_empty() {
            return;
        }
        self.dirty = true;
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    0 // stop at first
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn kill_selected(&mut self) {
        // Debounce: prevent rapid-fire kills
        const KILL_DEBOUNCE_MS: u64 = 200;
        if let Some(last) = self.last_kill_instant {
            if last.elapsed() < Duration::from_millis(KILL_DEBOUNCE_MS) {
                return;
            }
        }
        self.last_kill_instant = Some(Instant::now());

        if self.processes.is_empty() {
            self.message = Some("No process selected — list is empty".to_string());
            self.message_instant = Some(Instant::now());
            return;
        }

        let selected_info = self.table_state.selected()
            .and_then(|i| self.processes.get(i))
            .map(|p| (p.name.clone(), p.pid));

        let Some((ref name, pid)) = selected_info else {
            self.refresh();
            self.message = Some("No process selected".to_string());
            self.message_instant = Some(Instant::now());
            return;
        };

        // Clone what we need before any mutable borrow
        let name = name.clone();

        // Protect critical system PIDs and self
        if is_protected_pid(pid) {
            self.message = Some(format!("Refusing to kill protected PID {}", pid.as_u32()));
            self.message_instant = Some(Instant::now());
            return;
        }
        if Some(pid) == sysinfo::get_current_pid().ok() {
            self.message = Some("Refusing to kill self".to_string());
            self.message_instant = Some(Instant::now());
            return;
        }

        if let Some(process) = self.sys.process(pid) {
            let killed = process.kill();
            if killed {
                self.refresh();
                self.message = Some(format!("Killed process {} ({})", name, pid));
                self.message_instant = Some(Instant::now());
            } else {
                self.refresh();
                let gone = self.sys.process(pid).is_none();
                self.message = Some(if gone {
                    format!("Process {} ({}) already ended", name, pid)
                } else {
                    format!("Failed to kill process {} ({}). Try running with sudo?", name, pid)
                });
                self.message_instant = Some(Instant::now());
            }
        } else {
            self.refresh();
            self.message = Some(format!("Process {} ({}) no longer exists", name, pid.as_u32()));
            self.message_instant = Some(Instant::now());
        }
    }

    fn cycle_to(&mut self, next: SortColumn) {
        // Reset confirmation state on any sort action
        self.confirming_kill_all = false;
        self.toggle_sort(next);
    }

    /// Cycle to next sort column (Tab).
    pub fn cycle_sort_column(&mut self) {
        let next = match self.sort_column {
            SortColumn::Pid => SortColumn::Name,
            SortColumn::Name => SortColumn::Status,
            SortColumn::Status => SortColumn::Cpu,
            SortColumn::Cpu => SortColumn::Memory,
            SortColumn::Memory => SortColumn::Pid,
        };
        self.cycle_to(next);
    }

    /// Cycle to previous sort column (Shift+Tab).
    pub fn cycle_sort_column_reverse(&mut self) {
        let prev = match self.sort_column {
            SortColumn::Pid => SortColumn::Memory,
            SortColumn::Name => SortColumn::Pid,
            SortColumn::Status => SortColumn::Name,
            SortColumn::Cpu => SortColumn::Status,
            SortColumn::Memory => SortColumn::Cpu,
        };
        self.cycle_to(prev);
    }

    /// Reverse current sort direction.
    pub fn toggle_sort_direction(&mut self) {
        self.toggle_sort(self.sort_column);
    }

    pub fn toggle_sort(&mut self, column: SortColumn) {
        if self.sort_column == column {
            // Same column → toggle direction
            self.sort_direction = match self.sort_direction {
                SortDirection::Asc => SortDirection::Desc,
                SortDirection::Desc => SortDirection::Asc,
            };
        } else {
            // New column → start ascending
            self.sort_column = column;
            self.sort_direction = SortDirection::Asc;
        }
        self.apply_filter();
    }

    pub fn open_search(&mut self) {
        let Some(target) = self.table_state.selected().and_then(|i| self.processes.get(i)) else {
            self.message = Some("No process selected to search for".to_string());
            self.message_instant = Some(Instant::now());
            return;
        };
        self.dirty = true;
        let query = url_encode_query(&target.name);
        
        let (os_cmd, os_args, os_tag) = if cfg!(target_os = "windows") {
            ("cmd", vec!["/C", "start"], "windows")
        } else if cfg!(target_os = "macos") {
            ("open", vec![], "mac")
        } else {
            ("xdg-open", vec![], "linux")
        };

        let url = format!("https://www.google.com/search?q={}+process+{}", query, os_tag);
        
        let mut cmd = std::process::Command::new(os_cmd);
        for arg in os_args {
            cmd.arg(arg);
        }
        
        if let Err(e) = cmd.arg(&url).spawn() {
            self.message = Some(format!("Failed to open browser: {}", e));
            self.message_instant = Some(Instant::now());
        } else {
            self.message = Some(format!("Searching for process: {}", target.name));
            self.message_instant = Some(Instant::now());
        }
    }


    pub fn kill_all_wasteful(&mut self) {
        // Confirmation: first press sets flag, second press executes
        if !self.confirming_kill_all {
            self.confirming_kill_all = true;
            self.message = Some("Press K again to confirm killing all wasteful processes".to_string());
            self.message_instant = Some(Instant::now());
            return;
        }
        self.confirming_kill_all = false;

        let own_pid = sysinfo::get_current_pid().ok();
        let mut killed_count = 0;
        let mut targets: Vec<Pid> = Vec::new();

        // Identify wasteful processes first to avoid borrow checker issues with sys.processes()
        for (pid, process) in self.sys.processes() {
            // Never target self or protected PIDs
            if Some(*pid) == own_pid || is_protected_pid(*pid) {
                continue;
            }
            let cpu = process.cpu_usage();
            let status = process.status();
            let mem_mb = process.memory() as f64 / 1024.0 / 1024.0;
            
            // ponytail: Parked is macOS halted-at-clean-point, treated as idle
            let is_idle = cpu < IDLE_CPU_THRESHOLD && matches!(status,
                ProcessStatus::Sleep | ProcessStatus::Idle | ProcessStatus::Parked
            );
            if is_idle && mem_mb > WASTEFUL_MEM_MB {
                targets.push(*pid);
            }
        }

        let found_count = targets.len();
        for pid in targets {
            if let Some(process) = self.sys.process(pid) && process.kill() {
                killed_count += 1;
            }
        }

        self.refresh();
        self.message = Some(if killed_count > 0 {
            if killed_count == found_count {
                format!("Cleaned up {} wasteful processes", killed_count)
            } else {
                format!("Killed {}/{} wasteful processes ({} already exited)", killed_count, found_count, found_count - killed_count)
            }
        } else if found_count > 0 {
            format!("Could not kill any of {} wasteful processes (check permissions)", found_count)
        } else {
            "No wasteful processes found to clean up".to_string()
        });
        self.message_instant = Some(Instant::now());
    }
}

/// Skip PID 0, 1 (kernel/init/system-critical) and let caller handle self-pid.
fn is_protected_pid(pid: Pid) -> bool {
    let raw = pid.as_u32();
    raw == 0 || raw == 1
}

/// ponytail: minimal URL query encoding for search terms, no external crate
fn url_encode_query(s: &str) -> String {
    // ponytail: len*3 to avoid reallocation when non-ASCII bytes expand to %XX
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b' ' => out.push('+'),
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(b))
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
