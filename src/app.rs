use std::cmp::Ordering;
use std::time::{Duration, SystemTime};
use sysinfo::{Pid, ProcessStatus};

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

#[derive(Clone)]
pub struct ProcessInfo {
    pub pid: Pid,
    pub name: String,
    pub cpu: f32,
    pub mem_mb: f64,
    pub status: String,
}

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
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();
        
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        // Get boot time for uptime calculation
        let boot_time = SystemTime::UNIX_EPOCH + Duration::from_secs(sysinfo::System::boot_time());
        
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
            is_searching: false,
            search_query: String::new(),
        };
        
        // populate initial data immediately
        app.refresh();
        // second refresh to get CPU usage (sysinfo needs time delta between refreshes)
        // give it a tiny sleep to allow sysinfo to record a delta
        std::thread::sleep(Duration::from_millis(100));
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
                cpu: process.cpu_usage(),
                mem_mb: process.memory() as f64 / 1024.0 / 1024.0,
                status: match process.status() {
                    ProcessStatus::Run => "Running",
                    ProcessStatus::Sleep => "Sleeping",
                    ProcessStatus::Stop => "Stopped",
                    ProcessStatus::Zombie => "Zombie",
                    ProcessStatus::Idle => "Idle",
                    _ => "Other",
                }.to_string(),
            }
        }).collect();
    }

    /// Filter + sort from all_processes (in-memory, no I/O)
    pub fn apply_filter(&mut self) {
        let search_pattern = self.search_query.to_lowercase();

        self.processes = self.all_processes.iter().filter(|p| {
            search_pattern.is_empty() || p.name.to_lowercase().contains(&search_pattern)
        }).cloned().collect();

        self.processes.sort_by(|a, b| {
            let ordering = match self.sort_column {
                SortColumn::Pid => a.pid.as_u32().cmp(&b.pid.as_u32()),
                SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
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

        let len = self.processes.len();
        if len > 0 {
            if let Some(selected) = self.table_state.selected() {
                if selected >= len {
                    self.table_state.select(Some(len - 1));
                }
            } else {
                self.table_state.select(Some(0));
            }
        } else {
            self.table_state.select(None);
        }
    }

    pub fn refresh(&mut self) {
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
        if let Some(target) = self.table_state.selected().and_then(|i| self.processes.get(i))
            && let Some(process) = self.sys.process(target.pid)
        {
            let killed = process.kill();
            if killed {
                self.message = Some(format!("Killed process {} ({})", target.name, target.pid));
                std::thread::sleep(std::time::Duration::from_millis(100));
                self.refresh();
            } else {
                // On macOS, kill status is a bool. If false, it's likely a permission issue
                // or the process ended just before.
                self.message = Some(format!("Failed to kill process {} ({}). Try running with sudo?", target.name, target.pid));
            }
        }
    }

    pub fn open_search(&mut self) {
        if let Some(target) = self.table_state.selected().and_then(|i| self.processes.get(i)) {
            let query = urlencoding::encode(&target.name);
            
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
            } else {
                self.message = Some(format!("Searching for process: {}", target.name));
            }
        }
    }

    pub fn handle_click(&mut self, col: u16, row: u16, width: u16) {
        use crate::ui::*;
        // Layout adjusted:
        // chunks[0]: HEADER (height 2) -> y: 0, 1
        // chunks[1]: STATS (height 3)  -> y: 2, 3, 4
        // chunks[2]: TABLE (y: 5+)
        //   Table border: row 5
        //   Table content / header: row 6 
        if row == 6 { 
            // Header: check mouse x coordinate
            let fixed_width = 8 + 12 + 10 + 15 + 8 + (2 * 5); // PID+Status+CPU+Mem+Search + 5 separators
            let name_width = if width > fixed_width { width - fixed_width } else { COL_NAME_MIN_WIDTH };
            
            let mut current_x = 1; // Left border
            
            let pid_end = current_x + COL_PID_WIDTH;
            current_x = pid_end + 2; // Including spacing
            
            let name_end = current_x + name_width;
            current_x = name_end + 2; // Including spacing
            
            let status_end = current_x + COL_STATUS_WIDTH;
            current_x = status_end + 2; // Including spacing
            
            let cpu_end = current_x + COL_CPU_WIDTH;
            current_x = cpu_end + 2; // Including spacing
            
            let mem_end = current_x + COL_MEM_WIDTH;
            
            let clicked_col = if col >= 1 && col < pid_end {
                Some(SortColumn::Pid)
            } else if col >= pid_end + 2 && col < name_end {
                Some(SortColumn::Name)
            } else if col >= name_end + 2 && col < status_end {
                Some(SortColumn::Status)
            } else if col >= status_end + 2 && col < cpu_end {
                Some(SortColumn::Cpu)
            } else if col >= cpu_end + 2 && col < mem_end {
                Some(SortColumn::Memory)
            } else {
                None
            };

            if let Some(c) = clicked_col {
                if self.sort_column == c {
                    self.sort_direction = match self.sort_direction {
                        SortDirection::Asc => SortDirection::Desc,
                        SortDirection::Desc => SortDirection::Asc,
                    };
                } else {
                    self.sort_column = c;
                    self.sort_direction = SortDirection::Desc;
                }
                self.refresh();
            }
        }
    }

    pub fn kill_all_wasteful(&mut self) {
        let mut killed_count = 0;
        let mut targets = Vec::new();

        // Identify wasteful processes first to avoid borrow checker issues with sys.processes()
        for (pid, process) in self.sys.processes() {
            let cpu = process.cpu_usage();
            let status = process.status();
            let mem_mb = process.memory() as f64 / 1024.0 / 1024.0;
            
            let is_idle = cpu < 0.1 && (status == ProcessStatus::Sleep || status == ProcessStatus::Idle);
            if is_idle && mem_mb > 50.0 {
                targets.push((*pid, process.name().to_string_lossy().to_string()));
            }
        }

        for (pid, _name) in targets {
            if let Some(process) = self.sys.process(pid) && process.kill() {
                killed_count += 1;
            }
        }

        if killed_count > 0 {
            self.message = Some(format!("Cleaned up {} wasteful processes", killed_count));
            std::thread::sleep(std::time::Duration::from_millis(100));
            self.refresh();
        } else {
            self.message = Some("No wasteful processes found to clean up".to_string());
        }
    }
}
