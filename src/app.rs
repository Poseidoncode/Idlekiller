use std::cmp::Ordering;
use std::fmt::Write;
use std::time::{Duration, Instant, SystemTime};
use sysinfo::{Pid, Process, ProcessStatus, Uid, UpdateKind};

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
    pub wasteful_targets: Vec<Pid>,
    last_kill_instant: Option<Instant>,
    child_processes: Vec<std::process::Child>,
    current_effective_uid: Option<Uid>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let sys = sysinfo::System::new_all();

        let current_effective_uid = sysinfo::get_current_pid()
            .ok()
            .and_then(|pid| sys.process(pid))
            .and_then(|p| p.effective_user_id().cloned());

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
            wasteful_targets: Vec::new(),
            last_kill_instant: None,
            child_processes: Vec::new(),
            current_effective_uid,
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
            // ponytail: only cpu+mem+user, skip disk/tasks/env etc
            sysinfo::ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_user(UpdateKind::OnlyIfNotSet)
                .without_tasks(),
        );
        self.update_system_stats();

        self.all_processes = self
            .sys
            .processes()
            .iter()
            .map(|(pid, process)| {
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
                    }
                    .to_string(),
                }
            })
            .collect();
    }

    /// Filter + sort from all_processes (in-memory, no I/O)
    pub fn apply_filter(&mut self) {
        self.dirty = true;
        let search_pattern = self.search_query.to_lowercase();

        // Save selected PID before filtering/sorting so selection follows the process
        let selected_pid = self
            .table_state
            .selected()
            .and_then(|i| self.processes.get(i))
            .map(|p| p.pid);

        self.processes.clear();
        self.processes.extend(
            self.all_processes
                .iter()
                .filter(|p| search_pattern.is_empty() || p.name_lower.contains(&search_pattern))
                .cloned(),
        );

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
            .or(if self.processes.is_empty() {
                None
            } else {
                Some(0)
            });
        self.table_state.select(new_selected);
    }

    pub fn refresh(&mut self) {
        self.reap_children();
        // Keep messages visible for at least 4 seconds
        let expired = self
            .message_instant
            .map(|t| t.elapsed() >= Duration::from_secs(4))
            .unwrap_or(true);
        if expired {
            self.message = None;
            self.message_instant = None;
        }
        self.refresh_data();
        self.apply_filter();
    }

    fn reap_children(&mut self) {
        let mut i = 0;
        while i < self.child_processes.len() {
            match self.child_processes[i].try_wait() {
                Ok(None) => i += 1,
                Ok(Some(_)) => {
                    let _ = self.child_processes.remove(i).wait();
                }
                Err(_) => {
                    // If we cannot determine the state, reap the handle to avoid
                    // zombies without blocking the UI.
                    let _ = self.child_processes.remove(i).wait();
                }
            }
        }
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
        let i = i.min(self.processes.len().saturating_sub(1));
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
        let i = i.min(self.processes.len().saturating_sub(1));
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

        let selected_info = self
            .table_state
            .selected()
            .and_then(|i| self.processes.get(i))
            .map(|p| (p.name.clone(), p.pid));

        let Some((name, pid)) = selected_info else {
            self.refresh();
            self.message = Some("No process selected".to_string());
            self.message_instant = Some(Instant::now());
            return;
        };

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

        if is_protected_name(&name) {
            self.message = Some(format!(
                "Refusing to kill protected process {} ({})",
                name,
                pid.as_u32()
            ));
            self.message_instant = Some(Instant::now());
            return;
        }

        if let Some(process) = self.sys.process(pid) {
            if is_kernel_thread(process)
                || !is_owned_by_current_user(self.current_effective_uid.as_ref(), process)
            {
                self.message = Some(format!(
                    "Refusing to kill {} ({}): not owned or protected",
                    name, pid
                ));
                self.message_instant = Some(Instant::now());
                return;
            }
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
                    format!(
                        "Failed to kill process {} ({}). Try running with sudo?",
                        name, pid
                    )
                });
                self.message_instant = Some(Instant::now());
            }
        } else {
            self.refresh();
            self.message = Some(format!(
                "Process {} ({}) no longer exists",
                name,
                pid.as_u32()
            ));
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
        let Some(target) = self
            .table_state
            .selected()
            .and_then(|i| self.processes.get(i))
        else {
            self.message = Some("No process selected to search for".to_string());
            self.message_instant = Some(Instant::now());
            return;
        };
        self.dirty = true;
        let query = url_encode_query(&target.name);

        let (os_cmd, os_args, os_tag) = if cfg!(target_os = "windows") {
            // Avoid cmd.exe %VAR% expansion by using rundll32 URL handler.
            ("rundll32", vec!["url.dll,FileProtocolHandler"], "windows")
        } else if cfg!(target_os = "macos") {
            ("open", vec![], "mac")
        } else {
            ("xdg-open", vec![], "linux")
        };

        let url = format!(
            "https://www.google.com/search?q={}+process+{}",
            query, os_tag
        );

        let mut cmd = std::process::Command::new(os_cmd);
        for arg in os_args {
            cmd.arg(arg);
        }

        match cmd.arg(&url).spawn() {
            Ok(child) => {
                self.child_processes.push(child);
                self.message = Some(format!("Searching for process: {}", target.name));
                self.message_instant = Some(Instant::now());
            }
            Err(e) => {
                self.message = Some(format!("Failed to open browser: {}", e));
                self.message_instant = Some(Instant::now());
            }
        }
    }

    pub fn kill_all_wasteful(&mut self) {
        if !self.confirming_kill_all {
            self.wasteful_targets = self.find_wasteful_targets();
            self.confirming_kill_all = true;
            self.message = Some(if self.wasteful_targets.is_empty() {
                "No wasteful processes found to clean up".to_string()
            } else {
                let names = self
                    .wasteful_targets
                    .iter()
                    .take(5)
                    .map(|pid| {
                        self.sys
                            .process(*pid)
                            .map(|p| p.name().to_string_lossy().to_string())
                            .unwrap_or_else(|| pid.to_string())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let extra = if self.wasteful_targets.len() > 5 {
                    format!(" and {} more", self.wasteful_targets.len() - 5)
                } else {
                    String::new()
                };
                format!(
                    "Press K again to kill {}: {}{}",
                    self.wasteful_targets.len(),
                    names,
                    extra
                )
            });
            self.message_instant = Some(Instant::now());
            return;
        }
        self.confirming_kill_all = false;

        // Recompute targets at confirmation time to avoid stale PIDs / reuse.
        let targets = self.find_wasteful_targets();
        self.wasteful_targets.clear();
        let found_count = targets.len();
        let mut killed_count = 0;
        for pid in targets {
            if let Some(process) = self.sys.process(pid)
                && process.kill()
            {
                killed_count += 1;
            }
        }

        self.refresh();
        self.message = Some(if killed_count > 0 {
            if killed_count == found_count {
                format!("Cleaned up {} wasteful processes", killed_count)
            } else {
                format!(
                    "Killed {}/{} wasteful processes ({} already exited)",
                    killed_count,
                    found_count,
                    found_count - killed_count
                )
            }
        } else if found_count > 0 {
            format!(
                "Could not kill any of {} wasteful processes (check permissions)",
                found_count
            )
        } else {
            "No wasteful processes found to clean up".to_string()
        });
        self.message_instant = Some(Instant::now());
    }

    /// Identify idle but memory-heavy processes, excluding protected PIDs and known safe apps.
    fn find_wasteful_targets(&self) -> Vec<Pid> {
        let own_pid = sysinfo::get_current_pid().ok();
        let mut targets = Vec::new();
        for (pid, process) in self.sys.processes() {
            if Some(*pid) == own_pid
                || is_protected_pid(*pid)
                || is_kernel_thread(process)
                || !is_owned_by_current_user(self.current_effective_uid.as_ref(), process)
            {
                continue;
            }
            let name = process.name().to_string_lossy().to_string();
            if is_protected_name(&name) {
                continue;
            }
            let cpu = process.cpu_usage();
            let status = process.status();
            let mem_mb = process.memory() as f64 / 1024.0 / 1024.0;

            // Parked is macOS halted-at-clean-point, treated as idle
            let is_idle = cpu < IDLE_CPU_THRESHOLD
                && matches!(
                    status,
                    ProcessStatus::Sleep | ProcessStatus::Idle | ProcessStatus::Parked
                );
            if is_idle && mem_mb > WASTEFUL_MEM_MB {
                targets.push(*pid);
            }
        }
        targets
    }
}

/// Skip PID 0, 1, 2 (kernel/init/kthreadd) and let caller handle self-pid.
fn is_protected_pid(pid: Pid) -> bool {
    let raw = pid.as_u32();
    raw <= 2
}

fn is_kernel_thread(process: &Process) -> bool {
    let name = process.name().to_string_lossy();
    let is_kt = name.starts_with('[') && name.ends_with(']');
    let is_kthreadd_child = process.parent() == Some(Pid::from(2usize));
    is_kt || is_kthreadd_child
}

fn is_owned_by_current_user(current: Option<&Uid>, process: &Process) -> bool {
    match (current, process.effective_user_id()) {
        (Some(cur), Some(proc)) => cur == proc,
        (None, None) => true,
        _ => false,
    }
}

// Known safe processes that should not be bulk-killed: terminals, shells, window/DE/dock, browsers, editors, messengers.
const PROTECTED_NAMES: &[&str] = &[
    "kernel",
    "init",
    "system",
    "systemd",
    "launchd",
    "svchost",
    "csrss",
    "wininit",
    "smss",
    "winlogon",
    "services",
    "lsass",
    "dwm",
    "audiodg",
    "spoolsv",
    "taskhost",
    "dllhost",
    "fontdrvhost",
    "searchindexer",
    "securityhealthservice",
    "waasmedic",
    "wermgr",
    "msmpeng",
    "loginwindow",
    "windowserver",
    "dock",
    "finder",
    "alacritty",
    "wezterm",
    "wezterm-gui",
    "kitty",
    "konsole",
    "gnome-shell",
    "kwin",
    "kwin_x11",
    "plasmashell",
    "xfwm4",
    "i3",
    "sway",
    "dwm",
    "explorer",
    "powershell",
    "pwsh",
    "cmd",
    "terminal",
    "windowsterminal",
    "iterm2",
    "iterm",
    "hyper",
    "tabby",
    "terminator",
    "tilix",
    "xterm",
    "rxvt",
    "urxvt",
    "st",
    "foot",
    "qterminal",
    "yakuake",
    "tilda",
    "guake",
    "chrome",
    "google chrome",
    "firefox",
    "safari",
    "msedge",
    "edge",
    "code",
    "code-oss",
    "vim",
    "nvim",
    "neovide",
    "emacs",
    "xcode",
    "notes",
    "notion",
    "discord",
    "slack",
    "teams",
    "wechat",
    "telegram",
    "spotify",
];

fn is_protected_name(name: &str) -> bool {
    let name = name.to_lowercase();
    PROTECTED_NAMES.iter().any(|&p| {
        name.split(|c: char| !c.is_alphanumeric())
            .any(|part| part == p)
    })
}

/// Minimal URL query encoding for search terms, no external crate.
fn url_encode_query(s: &str) -> String {
    // len*3 to avoid reallocation when non-ASCII bytes expand to %XX
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b' ' => out.push('+'),
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(b))
            }
            _ => {
                // String's fmt::Write never errors for this size.
                let _ = write!(out, "%{:02X}", b);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{is_protected_name, is_protected_pid, url_encode_query};
    use sysinfo::Pid;

    #[test]
    fn protected_pids() {
        assert!(is_protected_pid(Pid::from(0usize)));
        assert!(is_protected_pid(Pid::from(1usize)));
        assert!(!is_protected_pid(Pid::from(1234usize)));
    }

    #[test]
    fn protected_names_tokenized() {
        assert!(is_protected_name("Google Chrome"));
        assert!(is_protected_name("chrome.exe"));
        assert!(is_protected_name("kernel_task"));
        assert!(is_protected_name("terminal"));
        assert!(!is_protected_name("xterminal"));
        assert!(!is_protected_name("my_custom_app"));
    }

    #[test]
    fn url_encode_basic() {
        assert_eq!(url_encode_query("Google Chrome"), "Google+Chrome");
        assert_eq!(url_encode_query("hello&world"), "hello%26world");
        assert_eq!(url_encode_query("café"), "caf%C3%A9");
    }
}
