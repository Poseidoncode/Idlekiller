//! Main-thread UI state: a single owned process snapshot and an index view.
//! All OS process operations execute in `collector`, never inside the renderer.
use std::{fmt::Write, time::{Duration, Instant}};
use ratatui::widgets::TableState;
use sysinfo::{Pid, Process, Uid};
use crate::collector::{Snapshot, TerminateCommand, TerminationReport, WorkerHandle};

pub(crate) const IDLE_CPU_THRESHOLD: f32 = 0.1;
pub(crate) const WASTEFUL_MEM_MB: f64 = 500.0;
const CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_BROWSER_CHILDREN: usize = 8;
const MAX_BULK_TARGETS: usize = 4;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SortColumn { Pid, Name, Status, Cpu, Memory }
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SortDirection { Asc, Desc }

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: Pid,
    pub name: String,
    pub name_lower: String,
    pub cpu: f32,
    pub mem_mb: f64,
    pub status: &'static str,
    pub start_time: u64,
    pub candidate: bool,
    /// Snapshot-level hint only; termination is always re-authorized by the worker.
    pub can_terminate: bool,
}
#[derive(Debug, Default)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub ram_used_mb: f64,
    pub ram_total_mb: f64,
    pub uptime_seconds: u64,
    /// End-to-end collection and transformation duration, not a CPU benchmark.
    pub scan_duration_ms: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProcessIdentity {
    pub(crate) pid: Pid,
    pub(crate) name: String,
    pub(crate) start_time: u64,
}
impl ProcessIdentity {
    pub(crate) fn matches_process(&self, process: &Process) -> bool {
        self.pid == process.pid()
            && self.name == process.name().to_string_lossy().as_ref()
            && self.start_time == process.start_time()
    }
}
impl From<&ProcessInfo> for ProcessIdentity {
    fn from(value: &ProcessInfo) -> Self {
        Self { pid: value.pid, name: value.name.clone(), start_time: value.start_time }
    }
}

pub struct App {
    worker: WorkerHandle,
    /// Indices into `all_processes`; no cloned names or process records.
    pub processes: Vec<usize>,
    all_processes: Vec<ProcessInfo>,
    pub table_state: TableState,
    pub should_quit: bool,
    pub message: Option<String>,
    pub sort_column: SortColumn,
    pub sort_direction: SortDirection,
    pub system_stats: SystemStats,
    pub is_searching: bool,
    pub search_query: String,
    search_before_edit: Option<String>,
    pub dirty: bool,
    pub message_instant: Option<Instant>,
    pub confirming_kill_all: bool,
    pub wasteful_targets: Vec<Pid>,
    pending_single: Option<ProcessIdentity>,
    single_confirmation_at: Option<Instant>,
    pending_bulk: Vec<ProcessIdentity>,
    bulk_confirmation_at: Option<Instant>,
    action_in_flight: bool,
    action_report: Option<TerminationReport>,
    worker_failed: bool,
    started_at: Instant,
    last_snapshot_at: Option<Instant>,
    child_processes: Vec<std::process::Child>,
    current_effective_uid: Option<Uid>,
    own_pid: Option<Pid>,
    cpu_ready: bool,
    collector_delayed: bool,
}
impl Default for App { fn default() -> Self { Self::new() } }

impl App {
    pub fn new() -> Self {
        Self::with_worker(WorkerHandle::start())
    }

    #[cfg(test)]
    pub(crate) fn new_for_test() -> Self {
        Self::with_worker(WorkerHandle::inactive())
    }

    fn with_worker(worker: WorkerHandle) -> Self {
        let mut table_state = TableState::default();
        table_state.select(None);
        Self {
            worker, processes: Vec::new(), all_processes: Vec::new(),
            table_state, should_quit: false, message: Some("Collecting process data…".into()),
            sort_column: SortColumn::Memory, sort_direction: SortDirection::Desc,
            system_stats: SystemStats::default(), is_searching: false, search_query: String::new(),
            search_before_edit: None,
            dirty: true, message_instant: Some(Instant::now()), confirming_kill_all: false,
            wasteful_targets: Vec::new(), pending_single: None, single_confirmation_at: None,
            pending_bulk: Vec::new(), bulk_confirmation_at: None, action_in_flight: false,
            action_report: None, worker_failed: false, started_at: Instant::now(),
            last_snapshot_at: None, child_processes: Vec::new(),
            current_effective_uid: None, own_pid: None, cpu_ready: false,
            collector_delayed: false,
        }
    }

    /// Return the actual process behind a filtered/sorted view position.
    pub fn process_info(&self, view_index: usize) -> Option<&ProcessInfo> {
        self.processes.get(view_index).and_then(|&index| self.all_processes.get(index))
    }

    pub(crate) fn process_info_from_snapshot(&self, raw_index: usize) -> &ProcessInfo { &self.all_processes[raw_index] }

    pub fn total_process_count(&self) -> usize { self.all_processes.len() }
    pub fn cpu_settled(&self) -> bool { self.cpu_ready }
    pub fn action_in_flight(&self) -> bool { self.action_in_flight }
    pub fn has_action_report(&self) -> bool { self.action_report.is_some() }
    pub fn worker_unavailable(&self) -> bool { self.worker_failed }
    pub(crate) fn action_report(&self) -> Option<&TerminationReport> { self.action_report.as_ref() }
    /// Dismisses the result dialog; the next keypress is never a destructive action.
    pub fn dismiss_action_report(&mut self) -> bool {
        if self.action_report.take().is_some() {
            self.dirty = true;
            true
        } else { false }
    }
    pub fn snapshot_delayed(&self) -> bool {
        self.last_snapshot_at.map_or_else(
            || self.started_at.elapsed() >= Duration::from_secs(4),
            |at| at.elapsed() >= Duration::from_secs(4),
        )
    }
    /// A collector can stop responding while leaving its sender alive; redraw as
    /// soon as the stale/fresh status flips even when there are no new snapshots.
    fn update_collector_delay(&mut self) {
        let delayed = self.snapshot_delayed();
        if delayed != self.collector_delayed {
            self.collector_delayed = delayed;
            self.dirty = true;
        }
    }

    /// At most four identities, all explicitly listed in the confirmation dialog.
    pub fn confirmation_target_lines(&self) -> Vec<String> {
        let targets: Vec<&ProcessIdentity> = if self.confirming_kill_all {
            self.pending_bulk.iter().collect()
        } else {
            self.pending_single.iter().collect()
        };
        targets.iter().map(|id| {
            let clean: String = id.name.chars().filter(|c| !c.is_control()).take(12).collect();
            format!("PID {}: {}", id.pid, clean)
        }).collect()
    }

    fn apply_snapshot(&mut self, data: Snapshot) {
        let selected_identity = self.table_state.selected().and_then(|i| self.process_info(i))
            .map(|p| (p.pid, p.start_time));
        self.system_stats = data.stats;
        self.current_effective_uid = data.current_user;
        self.own_pid = data.own_pid;
        self.cpu_ready = data.cpu_ready;
        self.last_snapshot_at = Some(data.sampled_at);
        self.all_processes = data.entries; // move, no deep copy
        self.apply_filter_preserving(selected_identity);
        // A preview must not be silently transferred to another selected PID.
        if self.pending_single.as_ref().is_some_and(|id| {
            self.table_state.selected().and_then(|i| self.process_info(i))
                .is_none_or(|p| p.pid != id.pid || p.start_time != id.start_time)
        }) { self.cancel_pending_kills(); }
    }

    /// In-memory view rebuild. Only integers are copied or sorted.
    pub fn apply_filter(&mut self) {
        let chosen = self.table_state.selected().and_then(|i| self.process_info(i))
            .map(|p| (p.pid, p.start_time));
        self.apply_filter_preserving(chosen);
    }

    fn apply_filter_preserving(&mut self, chosen: Option<(Pid, u64)>) {
        let query = self.search_query.to_lowercase();
        self.processes.clear();
        self.processes.extend(self.all_processes.iter().enumerate()
            .filter(|(_, p)| query.is_empty() || p.name_lower.contains(&query))
            .map(|(i, _)| i));
        let entries = &self.all_processes;
        let column = self.sort_column;
        let direction = self.sort_direction;
        self.processes.sort_unstable_by(|&l, &r| {
            let a = &entries[l]; let b = &entries[r];
            let compared = match column {
                SortColumn::Pid => a.pid.as_u32().cmp(&b.pid.as_u32()),
                SortColumn::Name => a.name_lower.cmp(&b.name_lower),
                SortColumn::Status => a.status.cmp(b.status),
                SortColumn::Cpu => a.cpu.total_cmp(&b.cpu),
                SortColumn::Memory => a.mem_mb.total_cmp(&b.mem_mb),
            };
            let compared = compared.then_with(|| a.pid.as_u32().cmp(&b.pid.as_u32()));
            match direction { SortDirection::Asc => compared, SortDirection::Desc => compared.reverse() }
        });
        let selected = chosen.and_then(|(pid, started)| self.processes.iter().position(|&idx| {
            let p = &entries[idx]; p.pid == pid && p.start_time == started
        })).or_else(|| if self.processes.is_empty() { None } else { Some(0) });
        self.table_state.select(selected);
        self.dirty = true;
    }

    /// Never blocks on sysinfo. Called every 100 ms by the event loop.
    pub fn refresh(&mut self) {
        self.reap_children();
        loop {
            match self.worker.try_result() {
                Ok(report) => {
                    self.action_in_flight = false;
                    let summary = report.message();
                    self.action_report = Some(report);
                    self.notify(summary);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if !self.worker_failed {
                        self.worker_failed = true;
                        self.action_in_flight = false;
                        self.notify("Process worker stopped unexpectedly; actions are disabled");
                    }
                    break;
                }
            }
        }
        if let Some(snapshot) = self.worker.latest() {
            self.apply_snapshot(snapshot);
            if self.message.as_deref() == Some("Collecting process data…") {
                self.message = None;
                self.message_instant = None;
            }
        }
        self.update_collector_delay();
        if !self.has_pending_kills()
            && self.message_instant.is_some_and(|t| t.elapsed() >= Duration::from_secs(4))
        {
            self.message = None;
            self.message_instant = None;
            self.dirty = true;
        }
        if self.pending_single.is_some() && !self.has_pending_kills() { self.cancel_pending_kills(); }
        if self.confirming_kill_all && !self.has_pending_kills() { self.cancel_pending_kills(); }
    }

    fn reap_children(&mut self) {
        let mut i = 0;
        while i < self.child_processes.len() {
            match self.child_processes[i].try_wait() {
                Ok(Some(_)) => { drop(self.child_processes.swap_remove(i)); }
                Ok(None) => i += 1,
                Err(err) => {
                    // An invalid Child handle must not occupy one of the bounded
                    // launcher slots forever. Dropping it releases our handle.
                    drop(self.child_processes.swap_remove(i));
                    self.notify(format!("Could not observe browser launcher: {err}"));
                }
            }
        }
    }
    fn notify(&mut self, text: impl Into<String>) {
        self.message = Some(text.into());
        self.message_instant = Some(Instant::now());
        self.dirty = true;
    }

    pub fn next(&mut self) {
        if self.processes.is_empty() { return; }
        let i = self.table_state.selected().map_or(0, |i| (i + 1).min(self.processes.len() - 1));
        self.table_state.select(Some(i)); self.dirty = true;
    }
    pub fn previous(&mut self) {
        if self.processes.is_empty() { return; }
        let i = self.table_state.selected().unwrap_or(0).saturating_sub(1);
        self.table_state.select(Some(i)); self.dirty = true;
    }

    pub fn has_pending_kills(&self) -> bool {
        (self.pending_single.is_some() && self.single_confirmation_at.is_some_and(|at| at.elapsed() < CONFIRMATION_TIMEOUT))
            || (self.confirming_kill_all && self.bulk_confirmation_at.is_some_and(|at| at.elapsed() < CONFIRMATION_TIMEOUT))
    }
    pub fn cancel_pending_kills(&mut self) {
        if self.pending_single.is_some() || self.confirming_kill_all {
            self.dirty = true;
            if self.message.as_ref().is_some_and(|m| m.starts_with("Press ")) {
                self.message = None;
                self.message_instant = None;
            }
        }
        self.pending_single = None; self.single_confirmation_at = None;
        self.confirming_kill_all = false; self.pending_bulk.clear();
        self.bulk_confirmation_at = None; self.wasteful_targets.clear();
    }

    pub fn kill_selected(&mut self) {
        if self.worker_failed { self.notify("Process worker is unavailable"); return; }
        if self.snapshot_delayed() { self.notify("Process snapshot is stale; termination disabled"); return; }
        if self.action_in_flight { self.notify("Previous termination request is still being checked"); return; }
        let Some((target, can_terminate)) = self.table_state.selected()
            .and_then(|i| self.process_info(i))
            .map(|p| (ProcessIdentity::from(p), p.can_terminate))
        else { self.cancel_pending_kills(); self.notify("No process selected"); return; };
        if !can_terminate {
            self.cancel_pending_kills();
            self.notify("Process protected or owner could not be verified");
            return;
        }
        if self.own_pid.is_none() || self.current_effective_uid.is_none() || Some(target.pid) == self.own_pid
            || target.pid.as_u32() <= 2 {
            self.cancel_pending_kills(); self.notify("Cannot verify owner or protected process identity"); return;
        }
        let confirmed = self.pending_single.as_ref() == Some(&target)
            && self.single_confirmation_at.is_some_and(|at| at.elapsed() < CONFIRMATION_TIMEOUT);
        if !confirmed {
            self.cancel_pending_kills();
            self.pending_single = Some(target.clone());
            self.single_confirmation_at = Some(Instant::now());
            self.notify(format!("Press Enter/x again within 8s to request termination of {} ({})", target.name, target.pid));
            return;
        }
        self.cancel_pending_kills();
        self.submit(vec![target], false);
    }

    fn submit(&mut self, targets: Vec<ProcessIdentity>, bulk: bool) {
        if self.worker_failed { self.notify("Process worker is unavailable"); return; }
        if self.action_in_flight { self.notify("Previous termination request still in progress"); return; }
        match self.worker.submit(TerminateCommand { targets, bulk, confirmed_at: Instant::now() }) {
            Ok(()) => {
                self.action_report = None;
                self.action_in_flight = true;
                self.notify("Verifying identities and permissions in background…");
            }
            Err(err) => self.notify(err),
        }
    }

    pub fn kill_all_wasteful(&mut self) {
        if self.worker_failed { self.notify("Process worker is unavailable"); return; }
        if self.action_in_flight { self.notify("Previous termination request still in progress"); return; }
        if self.snapshot_delayed() || !self.cpu_ready {
            self.cancel_pending_kills(); self.notify("Waiting for current, settled CPU measurements"); return;
        }
        if self.own_pid.is_none() || self.current_effective_uid.is_none() {
            self.cancel_pending_kills(); self.notify("Owner identity unknown; bulk action disabled"); return;
        }
        let confirmed = self.confirming_kill_all
            && self.bulk_confirmation_at.is_some_and(|at| at.elapsed() < CONFIRMATION_TIMEOUT);
        if confirmed {
            let targets = std::mem::take(&mut self.pending_bulk);
            self.cancel_pending_kills(); self.submit(targets, true);
            return;
        }
        self.cancel_pending_kills();
        let candidates: Vec<ProcessIdentity> = self.processes.iter()
            .map(|&i| &self.all_processes[i])
            .filter(|p| p.candidate).map(ProcessIdentity::from).collect();
        if candidates.is_empty() { self.notify("No stable, verified cleanup candidates found"); return; }
        if candidates.len() > MAX_BULK_TARGETS {
            self.notify(format!("{} candidates exceed the safety limit of {}; narrow the scope", candidates.len(), MAX_BULK_TARGETS));
            return;
        }
        let preview = candidates.iter().take(4)
            .map(|p| format!("{} ({})", p.name, p.pid)).collect::<Vec<_>>().join(", ");
        let extra = if candidates.len() > 4 { format!(" and {} more", candidates.len() - 4) } else { String::new() };
        self.wasteful_targets = candidates.iter().map(|p| p.pid).collect();
        self.pending_bulk = candidates;
        self.confirming_kill_all = true;
        self.bulk_confirmation_at = Some(Instant::now());
        self.notify(format!("Press K again within 8s to terminate {} candidates: {}{}", self.pending_bulk.len(), preview, extra));
    }

    pub fn toggle_sort(&mut self, column: SortColumn) {
        self.cancel_pending_kills();
        if self.sort_column == column {
            self.sort_direction = match self.sort_direction { SortDirection::Asc => SortDirection::Desc, SortDirection::Desc => SortDirection::Asc };
        } else { self.sort_column = column; self.sort_direction = SortDirection::Asc; }
        self.apply_filter();
    }
    pub fn toggle_sort_direction(&mut self) { self.toggle_sort(self.sort_column); }
    pub fn cycle_sort_column(&mut self) {
        self.toggle_sort(match self.sort_column {
            SortColumn::Pid => SortColumn::Name, SortColumn::Name => SortColumn::Status,
            SortColumn::Status => SortColumn::Cpu, SortColumn::Cpu => SortColumn::Memory,
            SortColumn::Memory => SortColumn::Pid,
        });
    }
    pub fn cycle_sort_column_reverse(&mut self) {
        self.toggle_sort(match self.sort_column {
            SortColumn::Pid => SortColumn::Memory, SortColumn::Name => SortColumn::Pid,
            SortColumn::Status => SortColumn::Name, SortColumn::Cpu => SortColumn::Status,
            SortColumn::Memory => SortColumn::Cpu,
        });
    }
    /// An interrupted filter edit must restore its previous value.
    pub fn begin_search(&mut self) {
        if self.is_searching { return; }
        self.cancel_pending_kills();
        self.search_before_edit = Some(self.search_query.clone());
        self.is_searching = true;
        self.dirty = true;
    }

    pub fn finish_search(&mut self, commit: bool) {
        if !self.is_searching { return; }
        self.is_searching = false;
        if !commit {
            if let Some(previous) = self.search_before_edit.take() {
                self.search_query = previous;
                self.apply_filter();
                return;
            }
        }
        self.search_before_edit = None;
        self.dirty = true;
    }

    pub fn open_search(&mut self) {
        let Some(name) = self.table_state.selected().and_then(|i| self.process_info(i))
            .map(|p| p.name.clone()) else { self.notify("No process selected to look up"); return; };
        self.reap_children();
        if self.child_processes.len() >= MAX_BROWSER_CHILDREN {
            self.notify("Too many pending browser launchers; retry later"); return;
        }
        let (program, args, os_tag): (&str, &[&str], &str) = if cfg!(windows) {
            ("rundll32", &["url.dll,FileProtocolHandler"], "windows")
        } else if cfg!(target_os = "macos") { ("open", &[], "mac") }
        else { ("xdg-open", &[], "linux") };
        let url = format!("https://www.google.com/search?q={}+process+{}", url_encode_query(&name), os_tag);
        match std::process::Command::new(program).args(args).arg(url).spawn() {
            Ok(child) => { self.child_processes.push(child); self.notify(format!("Searching process: {}", name)); }
            Err(err) => self.notify(format!("Failed to open browser: {err}")),
        }
    }
}
fn url_encode_query(text: &str) -> String {
    let mut out = String::with_capacity(text.len().saturating_mul(3));
    for b in text.bytes() {
        match b {
            b' ' => out.push('+'),
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(char::from(b)),
            _ => { let _ = write!(out, "%{:02X}", b); }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_index_view_does_not_copy_names() {
        let mut app = App::new_for_test();
        app.all_processes = vec![
            ProcessInfo { pid: Pid::from(501usize), name: "Alpha".into(), name_lower: "alpha".into(), cpu: 1.0, mem_mb: 1.0, status: "Running", start_time: 1, candidate: false, can_terminate: true },
            ProcessInfo { pid: Pid::from(502usize), name: "Beta".into(), name_lower: "beta".into(), cpu: 2.0, mem_mb: 2.0, status: "Sleeping", start_time: 2, candidate: false, can_terminate: true },
        ];
        app.apply_filter();
        assert_eq!(app.process_info(0).unwrap().name, "Beta");
        let raw_ptr = app.all_processes[1].name.as_ptr();
        assert_eq!(app.process_info(0).unwrap().name.as_ptr(), raw_ptr);
        app.search_query = "alpha".into();
        app.apply_filter();
        assert_eq!(app.processes.len(), 1);
        assert_eq!(app.process_info(0).unwrap().pid, Pid::from(501usize));
    }
    #[test]
    fn unicode_search_and_deterministic_sort() {
        let mut app = App::new_for_test();
        app.all_processes = vec![ProcessInfo {
            pid: Pid::from(99usize), name: "測試工具".into(), name_lower: "測試工具".into(),
            cpu: f32::NAN, mem_mb: 4.0, status: "Other", start_time: 8, candidate: false, can_terminate: true,
        }];
        app.search_query = "測試".into(); app.apply_filter();
        assert_eq!(app.processes, vec![0]);
    }
    #[test]
    fn selection_follows_pid_and_start_time_across_reordered_snapshots() {
        let mut app = App::new_for_test();
        let alpha = ProcessInfo { pid: Pid::from(501usize), name: "Alpha".into(),
            name_lower: "alpha".into(), cpu: 1.0, mem_mb: 2.0,
            status: "Running", start_time: 50, candidate: false, can_terminate: true };
        let beta = ProcessInfo { pid: Pid::from(502usize), name: "Beta".into(),
            name_lower: "beta".into(), cpu: 2.0, mem_mb: 3.0,
            status: "Running", start_time: 51, candidate: false, can_terminate: true };
        app.all_processes = vec![alpha.clone(), beta.clone()];
        app.apply_filter();
        app.table_state.select(Some(1)); // Alpha by descending memory
        assert_eq!(app.process_info(1).unwrap().pid, alpha.pid);
        let mut next_alpha = alpha.clone();
        next_alpha.mem_mb = 100.0; // sort order swaps, identity stays constant
        app.apply_snapshot(Snapshot { entries: vec![beta, next_alpha],
            stats: SystemStats::default(), own_pid: None, current_user: None,
            cpu_ready: true, sampled_at: Instant::now() });
        assert_eq!(app.table_state.selected(), Some(0));
        assert_eq!(app.process_info(0).unwrap().pid, alpha.pid);
    }

    #[test]
    fn collector_freshness_flip_invalidates_the_frame() {
        let mut app = App::new_for_test();
        app.last_snapshot_at = Some(Instant::now() - Duration::from_secs(5));
        app.dirty = false;
        app.update_collector_delay();
        assert!(app.collector_delayed);
        assert!(app.dirty);
        app.dirty = false;
        app.update_collector_delay();
        assert!(!app.dirty, "no redraw without a state transition");
        app.last_snapshot_at = Some(Instant::now());
        app.update_collector_delay();
        assert!(!app.collector_delayed);
        assert!(app.dirty, "recovery must also redraw");
    }

    #[test]
    fn dismissing_report_consumes_one_key_and_is_idempotent() {
        let mut app = App::new_for_test();
        app.action_report = Some(TerminationReport::default());
        app.dirty = false;
        assert!(app.dismiss_action_report());
        assert!(app.dirty);
        assert!(!app.dismiss_action_report());
    }

    #[test]
    fn protected_process_does_not_enter_confirmation() {
        let mut app = App::new_for_test();
        app.all_processes = vec![ProcessInfo {
            pid: Pid::from(321usize), name: "protected".into(),
            name_lower: "protected".into(), cpu: 0.0, mem_mb: 700.0,
            status: "Sleeping", start_time: 1, candidate: false,
            can_terminate: false,
        }];
        app.apply_filter();
        app.kill_selected();
        assert!(!app.has_pending_kills());
        assert!(app.message.as_deref().unwrap_or_default().contains("protected"));
    }

    #[test]
    fn canceled_filter_edit_restores_previous_query() {
        let mut app = App::new_for_test();
        app.search_query = "original".into();
        app.begin_search();
        app.search_query = "temporary".into();
        app.finish_search(false);
        assert_eq!(app.search_query, "original");
        assert!(!app.is_searching);
        assert!(app.search_before_edit.is_none());
        app.begin_search();
        app.search_query = "committed".into();
        app.finish_search(true);
        assert_eq!(app.search_query, "committed");
    }

    #[test]
    fn query_escape_percent_encodes_unsafe_bytes() {
        assert_eq!(url_encode_query("a&b café"), "a%26b+caf%C3%A9");
    }
}
