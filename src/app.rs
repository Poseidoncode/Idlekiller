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

            match self.sort_direction {
                SortDirection::Asc => ordering,
                SortDirection::Desc => ordering.reverse(),
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
        if self.processes.is_empty() {
            return;
        }
        
        if let Some(selected) = self.table_state.selected() {
            if let Some(target) = self.processes.get(selected) {
                if let Some(process) = self.sys.process(target.pid) {
                    let killed = process.kill();
                    if killed {
                        self.message = Some(format!("Killed process {} ({})", target.name, target.pid));
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        self.refresh();
                    } else {
                        self.message = Some(format!("Failed to kill process {} ({})", target.name, target.pid));
                    }
                }
            }
        }
    }

    pub fn open_search(&mut self) {
        if let Some(selected) = self.table_state.selected() {
            if let Some(target) = self.processes.get(selected) {
                let query = target.name.replace(" ", "+");
                let url = format!("https://www.google.com/search?q={}+process+mac", query);
                if let Err(e) = std::process::Command::new("open").arg(&url).spawn() {
                    self.message = Some(format!("Failed to open browser: {}", e));
                } else {
                    self.message = Some(format!("Searching for process: {}", target.name));
                }
            }
        }
    }

    pub fn handle_click(&mut self, col: u16, row: u16, width: u16) {
        if row == 4 { // Header is typically at row 4
            let fixed_width = 8 + 12 + 10 + 15 + 8 + 10 + 2;
            let name_width = if width > fixed_width { width - fixed_width } else { 20 };
            
            let mut current_x = 1; // Left border
            
            let pid_end = current_x + 8;
            current_x = pid_end + 2;
            
            let name_end = current_x + name_width;
            current_x = name_end + 2;
            
            let status_end = current_x + 12;
            current_x = status_end + 2;
            
            let cpu_end = current_x + 10;
            current_x = cpu_end + 2;
            
            let mem_end = current_x + 15;
            
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
