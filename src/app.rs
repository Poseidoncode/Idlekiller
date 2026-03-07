use std::cmp::Ordering;
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

pub struct ProcessInfo {
    pub pid: Pid,
    pub name: String,
    pub cpu: f32,
    pub mem_mb: f64,
    pub status: String,
}

use ratatui::widgets::TableState;

pub struct App {
    sys: sysinfo::System,
    pub processes: Vec<ProcessInfo>,
    pub table_state: TableState,
    pub should_quit: bool,
    pub message: Option<String>,
    pub sort_column: SortColumn,
    pub sort_direction: SortDirection,
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

        Self {
            sys,
            processes: Vec::new(),
            table_state,
            should_quit: false,
            message: None,
            sort_column: SortColumn::Memory,
            sort_direction: SortDirection::Desc,
        }
    }

    pub fn refresh(&mut self) {
        // Refresh processes
        self.sys.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, sysinfo::ProcessRefreshKind::everything());
        
        let mut new_processes = Vec::new();

        for (pid, process) in self.sys.processes() {
            // Allow all processes to be listed just like Activity Monitor
            // Note: killing root processes will fail unless run with sudo

            let cpu = process.cpu_usage();
            let status = match process.status() {
                ProcessStatus::Run => "Running",
                ProcessStatus::Sleep => "Sleeping",
                ProcessStatus::Stop => "Stopped",
                ProcessStatus::Zombie => "Zombie",
                ProcessStatus::Idle => "Idle",
                _ => "Other",
            };

            // Calculate memory in MB
            let mem_mb = process.memory() as f64 / 1024.0 / 1024.0;
            
            new_processes.push(ProcessInfo {
                pid: *pid,
                name: process.name().to_string_lossy().to_string(),
                cpu,
                mem_mb,
                status: status.to_string(),
            });
        }

        // Sort processes based on the current column and direction
        new_processes.sort_by(|a, b| {
            let ordering = match self.sort_column {
                SortColumn::Pid => a.pid.as_u32().cmp(&b.pid.as_u32()),
                SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortColumn::Status => a.status.cmp(&b.status),
                SortColumn::Cpu => a.cpu.partial_cmp(&b.cpu).unwrap_or(Ordering::Equal),
                SortColumn::Memory => a.mem_mb.partial_cmp(&b.mem_mb).unwrap_or(Ordering::Equal),
            };

            // If same value, use PID as secondary sort key for stability
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

        self.processes = new_processes;

        // ensure selected is in bounds
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

    pub fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.processes.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.processes.len().saturating_sub(1)
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
        // Header starts at HEADER_AREA_HEIGHT + 1 (border height)
        if row == HEADER_AREA_HEIGHT + 1 { 
            let fixed_width = COL_PID_WIDTH + COL_STATUS_WIDTH + COL_CPU_WIDTH + COL_MEM_WIDTH + COL_SEARCH_WIDTH + 2;
            let name_width = if width > fixed_width { width - fixed_width } else { COL_NAME_MIN_WIDTH };
            
            let mut current_x = 1; // Left border
            
            let pid_end = current_x + COL_PID_WIDTH;
            current_x = pid_end + 2;
            
            let name_end = current_x + name_width;
            current_x = name_end + 2;
            
            let status_end = current_x + COL_STATUS_WIDTH;
            current_x = status_end + 2;
            
            let cpu_end = current_x + COL_CPU_WIDTH;
            current_x = cpu_end + 2;
            
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
}
