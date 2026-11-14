//! OS access is owned exclusively by a worker thread. The UI never scans or
//! signals processes. Snapshots live in a latest-value slot (capacity one),
//! so a slow terminal cannot cause an unbounded queue of outdated snapshots.
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, mpsc::{self, Receiver, SyncSender, TrySendError}},
    thread,
    time::{Duration, Instant},
};
use sysinfo::{Pid, Process, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System, Uid, UpdateKind};

use crate::app::{ProcessIdentity, ProcessInfo, SystemStats, IDLE_CPU_THRESHOLD, WASTEFUL_MEM_MB};

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const ACTION_MAX_AGE: Duration = Duration::from_secs(8);
const SNAPSHOT_MAX_AGE: Duration = Duration::from_secs(3);
const MIN_CPU_INTERVAL: Duration = Duration::from_millis(250);
const REQUIRED_IDLE_SAMPLES: u8 = 3;

pub(crate) struct Snapshot {
    pub(crate) entries: Vec<ProcessInfo>,
    pub(crate) stats: SystemStats,
    pub(crate) own_pid: Option<Pid>,
    pub(crate) current_user: Option<Uid>,
    pub(crate) cpu_ready: bool,
    pub(crate) sampled_at: Instant,
}

#[derive(Debug)]
pub(crate) struct TerminateCommand {
    pub(crate) targets: Vec<ProcessIdentity>,
    pub(crate) bulk: bool,
    pub(crate) confirmed_at: Instant,
}

#[derive(Default, Debug)]
pub(crate) struct TerminationReport {
    pub(crate) total: usize,
    pub(crate) requested: usize,
    pub(crate) missing: usize,
    pub(crate) changed: usize,
    pub(crate) denied: usize,
    pub(crate) failed: usize,
    pub(crate) expired: bool,
    /// Per-PID status for a readable, persistent result dialog.
    pub(crate) outcomes: Vec<(Pid, &'static str)>,
}

impl TerminationReport {
    pub(crate) fn message(&self) -> String {
        let attempted = self.requested + self.missing + self.changed + self.denied + self.failed;
        let remainder = self.total.saturating_sub(attempted);
        format!(
            "Signals: {}/{}; missing: {}, changed: {}, denied: {}, failed: {}, not attempted: {}{} (not exit confirmation)",
            self.requested, self.total, self.missing, self.changed, self.denied, self.failed,
            remainder, if self.expired { " [timed out]" } else { "" }
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Denial {
    Missing,
    IdentityChanged,
    UnverifiedOwner,
    Protected,
    NoLongerEligible,
    SignalFailed,
}

/// An independent, owned representation allows deterministic fake-source
/// tests. Process objects and OS handles never leave the worker thread.
#[derive(Clone)]
struct InspectedProcess {
    identity: ProcessIdentity,
    effective_user: Option<Uid>,
    kernel_thread: bool,
    sleeping: bool,
    memory_mb: f64,
    eligible_in_recent_sample: bool,
}

trait ProcessSource: Send + 'static {
    fn collect(&mut self) -> Snapshot;
    fn inspect(&mut self, pid: Pid) -> Option<InspectedProcess>;
    fn signal(&mut self, identity: &ProcessIdentity) -> bool;
    fn own_pid(&self) -> Option<Pid>;
    fn current_user(&self) -> Option<&Uid>;
}

/// This policy is reused by fake-source tests; no test sends a real signal.
fn terminate_checked(source: &mut impl ProcessSource, target: &ProcessIdentity, bulk: bool) -> Result<(), Denial> {
    let own = source.own_pid().ok_or(Denial::UnverifiedOwner)?;
    if target.pid == own || target.pid.as_u32() <= 2 || is_protected_name(&target.name) {
        return Err(Denial::Protected);
    }
    let user = source.current_user().cloned().ok_or(Denial::UnverifiedOwner)?;
    if bulk && is_superuser(&user) {
        return Err(Denial::Protected);
    }
    let observed = source.inspect(target.pid).ok_or(Denial::Missing)?;
    if observed.identity != *target {
        return Err(Denial::IdentityChanged);
    }
    if observed.kernel_thread || is_protected_name(&observed.identity.name) {
        return Err(Denial::Protected);
    }
    if observed.effective_user.as_ref() != Some(&user) {
        return Err(Denial::UnverifiedOwner);
    }
    if bulk && !(observed.eligible_in_recent_sample
        && observed.sleeping
        && observed.memory_mb > WASTEFUL_MEM_MB) {
        return Err(Denial::NoLongerEligible);
    }
    if source.signal(target) { Ok(()) } else { Err(Denial::SignalFailed) }
}

fn execute_command(source: &mut impl ProcessSource, cmd: TerminateCommand) -> TerminationReport {
    let mut report = TerminationReport { total: cmd.targets.len(), ..TerminationReport::default() };
    // The OS scan can take arbitrarily long. Expired commands must not execute.
    if cmd.confirmed_at.elapsed() >= ACTION_MAX_AGE {
        report.expired = true;
        return report;
    }
    for target in cmd.targets {
        if cmd.confirmed_at.elapsed() >= ACTION_MAX_AGE {
            report.expired = true;
            break;
        }
        let status = match terminate_checked(source, &target, cmd.bulk) {
            Ok(()) => { report.requested += 1; "signal requested" },
            Err(Denial::Missing) => { report.missing += 1; "no longer present" },
            Err(Denial::IdentityChanged) => { report.changed += 1; "identity changed" },
            Err(Denial::SignalFailed) => { report.failed += 1; "signal failed" },
            Err(Denial::UnverifiedOwner) => { report.denied += 1; "owner unverified" },
            Err(Denial::Protected) => { report.denied += 1; "protected" },
            Err(Denial::NoLongerEligible) => { report.denied += 1; "no longer eligible" },
        };
        report.outcomes.push((target.pid, status));
    }
    report
}

fn poll_remaining(cycle_started: Instant, now: Instant) -> Duration {
    (cycle_started + POLL_INTERVAL).saturating_duration_since(now)
}

pub(crate) struct WorkerHandle {
    slot: Arc<Mutex<Option<Snapshot>>>,
    command_tx: SyncSender<TerminateCommand>,
    result_rx: Receiver<TerminationReport>,
}

impl WorkerHandle {
    /// Build a disconnected handle for app-state tests without starting OS scanners.
    #[cfg(test)]
    pub(crate) fn inactive() -> Self {
        let (command_tx, command_rx) = mpsc::sync_channel(1);
        let (result_tx, result_rx) = mpsc::channel();
        drop(command_rx);
        drop(result_tx);
        Self { slot: Arc::new(Mutex::new(None)), command_tx, result_rx }
    }

    pub(crate) fn start() -> Self {
        Self::start_with_source(SysinfoSource::new())
    }

    fn start_with_source(mut source: impl ProcessSource) -> Self {
        let slot = Arc::new(Mutex::new(None));
        let worker_slot = Arc::clone(&slot);
        // At most one action can be queued. UI also prevents a second pending request.
        let (command_tx, command_rx) = mpsc::sync_channel::<TerminateCommand>(1);
        let (result_tx, result_rx) = mpsc::channel::<TerminationReport>();
        thread::spawn(move || {
            loop {
                let cycle_started = Instant::now();
                let snapshot = source.collect();
                if let Ok(mut latest) = worker_slot.lock() {
                    *latest = Some(snapshot); // replace an unconsumed stale snapshot
                } else { break; }

                // Do not append an extra whole second to the scan duration.
                // When collection exceeds the interval, check the command
                // queue without blocking so actions cannot be starved.
                let remaining = poll_remaining(cycle_started, Instant::now());
                let command = if remaining.is_zero() {
                    match command_rx.try_recv() {
                        Ok(cmd) => Some(cmd),
                        Err(mpsc::TryRecvError::Empty) => None,
                        Err(mpsc::TryRecvError::Disconnected) => return,
                    }
                } else {
                    match command_rx.recv_timeout(remaining) {
                        Ok(cmd) => Some(cmd),
                        Err(mpsc::RecvTimeoutError::Timeout) => None,
                        Err(mpsc::RecvTimeoutError::Disconnected) => return,
                    }
                };
                if let Some(cmd) = command {
                    let result = execute_command(&mut source, cmd);
                    if result_tx.send(result).is_err() { return; }
                    // A completed termination triggers the next scan immediately.
                }
            }
        });
        Self { slot, command_tx, result_rx }
    }

    pub(crate) fn latest(&self) -> Option<Snapshot> {
        self.slot.lock().ok()?.take()
    }

    pub(crate) fn submit(&self, command: TerminateCommand) -> Result<(), &'static str> {
        match self.command_tx.try_send(command) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err("Worker is busy; retry after the previous action"),
            Err(TrySendError::Disconnected(_)) => Err("Process worker is unavailable"),
        }
    }

    pub(crate) fn try_result(&self) -> Result<TerminationReport, mpsc::TryRecvError> {
        self.result_rx.try_recv()
    }
}

struct SysinfoSource {
    sys: System,
    own: Option<Pid>,
    uid: Option<Uid>,
    sample_count: u64,
    previous_sample_at: Option<Instant>,
    consecutive: HashMap<(Pid, u64), u8>,
    recently_eligible: HashSet<(Pid, u64)>,
    sampled_at: Option<Instant>,
}

impl SysinfoSource {
    fn new() -> Self {
        Self {
            sys: System::new(),
            own: sysinfo::get_current_pid().ok(),
            uid: None,
            sample_count: 0,
            previous_sample_at: None,
            consecutive: HashMap::new(),
            recently_eligible: HashSet::new(),
            sampled_at: None,
        }
    }
}

impl ProcessSource for SysinfoSource {
    fn collect(&mut self) -> Snapshot {
        let scan_start = Instant::now();
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu().with_memory()
                .with_user(UpdateKind::OnlyIfNotSet).without_tasks(),
        );
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        if self.uid.is_none() {
            self.uid = self.own.and_then(|pid| self.sys.process(pid))
                .and_then(|p| verified_user_id(p).cloned());
        }
        let at = Instant::now();
        let spaced = self.previous_sample_at.is_none_or(|previous| at.saturating_duration_since(previous) >= MIN_CPU_INTERVAL);
        if spaced {
            self.sample_count = self.sample_count.saturating_add(1);
            self.previous_sample_at = Some(at);
        }
        let cpu_ready = self.sample_count >= 2;
        let privileged_user = self.uid.as_ref().is_some_and(is_superuser);
        let mut next = HashMap::with_capacity(self.sys.processes().len());
        let mut eligible = HashSet::new();
        let entries = self.sys.processes().iter().map(|(&pid, process)| {
            let name = process.name().to_string_lossy().into_owned();
            let name_lower = name.to_lowercase();
            let key = (pid, process.start_time());
            let can_terminate = !is_protected_pid(pid) && Some(pid) != self.own
                && !is_kernel_thread(process) && !is_protected_lower(&name_lower)
                && self.uid.as_ref().is_some_and(|uid| verified_user_id(process) == Some(uid));
            let eligible_now = cpu_ready && process.cpu_usage().is_finite()
                && process.cpu_usage() < IDLE_CPU_THRESHOLD
                && is_sleeping(process.status()) && (process.memory() as f64 / 1_048_576.0) > WASTEFUL_MEM_MB
                && can_terminate && !privileged_user;
            let streak = next_streak(
                self.consecutive.get(&key).copied().unwrap_or(0), spaced, eligible_now
            );
            if streak > 0 { next.insert(key, streak); }
            let candidate = eligible_now && streak >= REQUIRED_IDLE_SAMPLES;
            if candidate { eligible.insert(key); }
            ProcessInfo {
                pid,
                name_lower,
                name,
                cpu: if process.cpu_usage().is_finite() { process.cpu_usage().max(0.0) } else { 0.0 },
                mem_mb: process.memory() as f64 / 1_048_576.0,
                status: status_label(process.status()),
                start_time: process.start_time(),
                candidate,
                can_terminate,
            }
        }).collect();
        self.consecutive = next;
        self.recently_eligible = eligible;
        self.sampled_at = Some(at);
        let uptime_seconds = System::uptime();
        Snapshot {
            entries,
            stats: SystemStats {
                cpu_usage: if self.sys.global_cpu_usage().is_finite() {
                    self.sys.global_cpu_usage().max(0.0)
                } else { 0.0 },
                ram_used_mb: self.sys.used_memory() as f64 / 1_048_576.0,
                ram_total_mb: self.sys.total_memory() as f64 / 1_048_576.0,
                uptime_seconds,
                scan_duration_ms: u64::try_from(scan_start.elapsed().as_millis()).unwrap_or(u64::MAX),
            },
            own_pid: self.own,
            current_user: self.uid.clone(),
            cpu_ready,
            sampled_at: at,
        }
    }

    fn inspect(&mut self, pid: Pid) -> Option<InspectedProcess> {
        // Re-read ownership, existence, and status before every signal.
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing().with_memory().with_user(UpdateKind::Always).without_tasks(),
        );
        let process = self.sys.process(pid)?;
        let key = (pid, process.start_time());
        Some(InspectedProcess {
            identity: ProcessIdentity {
                pid,
                name: process.name().to_string_lossy().into_owned(),
                start_time: process.start_time(),
            },
            effective_user: verified_user_id(process).cloned(),
            kernel_thread: is_kernel_thread(process),
            sleeping: is_sleeping(process.status()),
            memory_mb: process.memory() as f64 / 1_048_576.0,
            eligible_in_recent_sample: self.sampled_at.is_some_and(|at| at.elapsed() < SNAPSHOT_MAX_AGE)
                && self.recently_eligible.contains(&key),
        })
    }

    fn signal(&mut self, target: &ProcessIdentity) -> bool {
        self.sys.process(target.pid).is_some_and(|process| {
            // Narrow, but cannot eliminate, PID reuse races without OS handles.
            target.matches_process(process) && request_termination(process)
        })
    }

    fn own_pid(&self) -> Option<Pid> { self.own }
    fn current_user(&self) -> Option<&Uid> { self.uid.as_ref() }
}

fn next_streak(previous: u8, spaced: bool, eligible: bool) -> u8 {
    if !spaced { previous }
    else if eligible { previous.saturating_add(1) }
    else { 0 }
}

fn status_label(status: ProcessStatus) -> &'static str {
    match status {
        ProcessStatus::Run => "Running",
        ProcessStatus::Sleep => "Sleeping",
        ProcessStatus::Stop => "Stopped",
        ProcessStatus::Zombie => "Zombie",
        ProcessStatus::Idle | ProcessStatus::Parked => "Idle",
        _ => "Other",
    }
}
fn is_sleeping(status: ProcessStatus) -> bool {
    matches!(status, ProcessStatus::Sleep | ProcessStatus::Idle | ProcessStatus::Parked)
}
fn is_protected_pid(pid: Pid) -> bool { pid.as_u32() <= 2 }
/// `effective_user_id` is unavailable on Windows; `user_id` is the
/// best available identity there. Unknown IDs still fail closed.
fn verified_user_id(process: &Process) -> Option<&Uid> {
    #[cfg(windows)]
    { process.user_id() }
    #[cfg(not(windows))]
    { process.effective_user_id() }
}
fn is_kernel_thread(process: &Process) -> bool {
    let name = process.name().to_string_lossy();
    (name.starts_with('[') && name.ends_with(']'))
        || process.parent() == Some(Pid::from(2usize))
        || process.thread_kind() == Some(sysinfo::ThreadKind::Kernel)
}
#[cfg(not(windows))]
fn is_superuser(uid: &Uid) -> bool {
    use std::ops::Deref;
    *uid.deref() == 0
}
#[cfg(windows)]
fn is_superuser(uid: &Uid) -> bool {
    use std::str::FromStr;
    // The three service account identities must never be bulk-cleanup actors.
    // This does not replace a Windows elevated-token check; see V3 report.
    ["S-1-5-18", "S-1-5-19", "S-1-5-20"]
        .into_iter()
        .filter_map(|sid| Uid::from_str(sid).ok())
        .any(|account| &account == uid)
}

const PROTECTED_NAMES: &[&str] = &[
    "kernel", "init", "system", "systemd", "launchd", "svchost", "csrss", "wininit",
    "smss", "winlogon", "services", "lsass", "dwm", "audiodg", "spoolsv",
    "taskhost", "dllhost", "fontdrvhost", "searchindexer", "securityhealthservice",
    "waasmedic", "wermgr", "msmpeng", "loginwindow", "windowserver", "dock", "finder",
    "alacritty", "wezterm", "kitty", "konsole", "gnome-shell", "kwin", "plasmashell",
    "xfwm4", "i3", "sway", "explorer", "powershell", "pwsh", "cmd", "sh", "bash",
    "zsh", "fish", "nu", "dash", "elvish", "xonsh", "ssh", "sshd", "scp", "sftp",
    "mosh", "tmux", "screen", "docker", "dockerd", "containerd", "podman",
    "dbus", "pipewire", "wireplumber", "pulseaudio", "cron", "terminal",
    "windowsterminal", "iterm2", "iterm", "hyper", "tabby", "xterm", "foot", "chrome",
    "google chrome", "firefox", "safari", "msedge", "edge", "code", "code-oss",
    "vim", "nvim", "neovide", "emacs", "xcode", "notes", "notion", "discord",
    "slack", "teams", "wechat", "telegram", "spotify",
];

fn is_protected_name(name: &str) -> bool {
    is_protected_lower(&name.to_lowercase())
}
fn is_protected_lower(lower: &str) -> bool {
    // Tokenize once per process rather than repeating a Unicode split for
    // every entry in the protected list on every snapshot.
    if PROTECTED_NAMES.iter().any(|&name| name.contains(' ') && lower == name) {
        return true;
    }
    lower.split(|ch: char| !ch.is_alphanumeric()).any(|part| {
        !part.is_empty() && PROTECTED_NAMES.iter().any(|&name| {
            !name.contains(' ') && part == name
        })
    })
}

fn request_termination(process: &Process) -> bool {
    #[cfg(unix)]
    { process.kill_with(sysinfo::Signal::Term).unwrap_or(false) }
    #[cfg(not(unix))]
    { process.kill() }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeSource {
        own: Option<Pid>,
        user: Option<Uid>,
        found: Option<InspectedProcess>,
        signals: usize,
    }
    impl ProcessSource for FakeSource {
        fn collect(&mut self) -> Snapshot { panic!("not needed by fake") }
        fn inspect(&mut self, _pid: Pid) -> Option<InspectedProcess> { self.found.clone() }
        fn signal(&mut self, _identity: &ProcessIdentity) -> bool { self.signals += 1; true }
        fn own_pid(&self) -> Option<Pid> { self.own }
        fn current_user(&self) -> Option<&Uid> { self.user.as_ref() }
    }
    fn test_uid(alternate: bool) -> Uid {
        #[cfg(windows)]
        {
            use std::str::FromStr;
            Uid::from_str(if alternate {
                "S-1-5-21-1-2-3-1001"
            } else {
                "S-1-5-21-1-2-3-1000"
            }).unwrap()
        }
        #[cfg(not(windows))]
        {
            Uid::try_from(if alternate { 2000usize } else { 1000usize }).unwrap()
        }
    }
    fn fixture() -> (FakeSource, ProcessIdentity) {
        let target = ProcessIdentity {pid: Pid::from(54321usize), name: "test-harmless".into(), start_time: 100};
        let user = test_uid(false);
        let src = FakeSource {
            own: Some(Pid::from(999usize)), user: Some(user.clone()),
            found: Some(InspectedProcess { identity: target.clone(), effective_user: Some(user),
                kernel_thread: false, sleeping: true, memory_mb: 900.0, eligible_in_recent_sample: true }),
            signals: 0,
        };
        (src, target)
    }

    #[test]
    fn pid_reused_different_start_time_never_signalled() {
        let (mut src, target) = fixture();
        src.found.as_mut().unwrap().identity.start_time = 101;
        assert_eq!(terminate_checked(&mut src, &target, false), Err(Denial::IdentityChanged));
        assert_eq!(src.signals, 0);
    }
    #[test]
    fn pid_reused_different_name_never_signalled() {
        let (mut src, target) = fixture();
        src.found.as_mut().unwrap().identity.name = "other".into();
        assert_eq!(terminate_checked(&mut src, &target, false), Err(Denial::IdentityChanged));
        assert_eq!(src.signals, 0);
    }
    #[test]
    fn ownership_unknown_or_different_never_signalled() {
        let (mut src, target) = fixture();
        src.found.as_mut().unwrap().effective_user = None;
        assert_eq!(terminate_checked(&mut src, &target, false), Err(Denial::UnverifiedOwner));
        src.found.as_mut().unwrap().effective_user = Some(test_uid(true));
        assert_eq!(terminate_checked(&mut src, &target, false), Err(Denial::UnverifiedOwner));
        src.user = None;
        assert_eq!(terminate_checked(&mut src, &target, false), Err(Denial::UnverifiedOwner));
        assert_eq!(src.signals, 0);
    }
    #[test]
    fn missing_and_protected_never_signalled() {
        let (mut src, target) = fixture();
        src.found = None;
        assert_eq!(terminate_checked(&mut src, &target, false), Err(Denial::Missing));
        let (mut src, target) = fixture();
        src.found.as_mut().unwrap().kernel_thread = true;
        assert_eq!(terminate_checked(&mut src, &target, false), Err(Denial::Protected));
        assert_eq!(src.signals, 0);
    }
    #[test]
    fn bulk_requires_recent_stable_candidate() {
        let (mut src, target) = fixture();
        src.found.as_mut().unwrap().eligible_in_recent_sample = false;
        assert_eq!(terminate_checked(&mut src, &target, true), Err(Denial::NoLongerEligible));
        src.found.as_mut().unwrap().eligible_in_recent_sample = true;
        src.found.as_mut().unwrap().sleeping = false;
        assert_eq!(terminate_checked(&mut src, &target, true), Err(Denial::NoLongerEligible));
        assert_eq!(src.signals, 0);
    }
    #[test]
    fn only_matching_verified_target_can_receive_signal() {
        let (mut src, target) = fixture();
        assert_eq!(terminate_checked(&mut src, &target, true), Ok(()));
        assert_eq!(src.signals, 1);
    }
    #[test]
    fn per_pid_report_shows_the_failure_reason_without_signalling() {
        let (mut src, target) = fixture();
        src.found = None;
        let report = execute_command(&mut src, TerminateCommand {
            targets: vec![target.clone()], bulk: false,
            confirmed_at: Instant::now(),
        });
        assert_eq!(src.signals, 0);
        assert_eq!(report.missing, 1);
        assert_eq!(report.outcomes, vec![(target.pid, "no longer present")]);
    }

    #[test]
    fn expired_confirmation_cannot_signal() {
        let (mut src, target) = fixture();
        let result = execute_command(&mut src, TerminateCommand {
            targets: vec![target], bulk: true,
            confirmed_at: Instant::now() - Duration::from_secs(10),
        });
        assert!(result.expired);
        assert_eq!(src.signals, 0);
    }
    #[test]
    fn three_consecutive_samples_required_and_reuse_resets_streak() {
        let first = next_streak(0, true, true);
        let second = next_streak(first, true, true);
        let third = next_streak(second, true, true);
        assert!(second < REQUIRED_IDLE_SAMPLES);
        assert!(third >= REQUIRED_IDLE_SAMPLES);
        assert_eq!(next_streak(third, true, false), 0);
        assert_eq!(next_streak(first, false, true), first);
        // New PID/start-time keys are absent from the map, i.e. start at zero.
        assert_eq!(next_streak(0, true, true), 1);
    }
    #[test]
    fn timeout_report_never_claims_zero_sent_if_partial() {
        let report = TerminationReport { total: 3, requested: 1, expired: true, ..TerminationReport::default() };
        let message = report.message();
        assert!(message.contains("Signals: 1/3"));
        assert!(message.contains("not attempted: 2"));
        assert!(message.contains("timed out"));
    }
    #[cfg(windows)]
    #[test]
    fn service_account_cannot_execute_bulk_cleanup() {
        use std::str::FromStr;
        for sid in ["S-1-5-18", "S-1-5-19", "S-1-5-20"] {
            assert!(is_superuser(&Uid::from_str(sid).unwrap()));
        }
        assert!(!is_superuser(&test_uid(false)));
    }

    #[test]
    fn polling_interval_includes_scan_time_and_expires_safely() {
        let cycle = Instant::now();
        assert_eq!(poll_remaining(cycle, cycle + Duration::from_millis(250)),
            Duration::from_millis(750));
        assert_eq!(poll_remaining(cycle, cycle + Duration::from_secs(2)), Duration::ZERO);
    }

    #[test]
    fn protected_process_name_is_case_insensitive() {
        assert!(is_protected_name("chrome.exe"));
        assert!(is_protected_name("Google Chrome"));
        assert!(is_protected_name("SSH-session"));
        assert!(!is_protected_name("chromehelper"));
        assert!(!is_protected_name("xterminal"));
    }
}
