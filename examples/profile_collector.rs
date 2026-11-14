//! Non-destructive collector/RSS profiler for release builds.
//! Usage: cargo run --release --example profile_collector -- 3600 > profile.csv
//! Only reads process metadata; never submits a termination command.
use idlekiller::app::App;
use std::{env, thread, time::{Duration, Instant}};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

fn main() {
    let duration_seconds = env::args()
        .nth(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60)
        .clamp(1, 21_600);
    let my_pid = sysinfo::get_current_pid().expect("profile process PID");
    let mut meter = System::new();
    let mut app = App::new();
    let started = Instant::now();
    println!("seconds,process_count,scan_ms,own_rss_bytes");
    while started.elapsed() < Duration::from_secs(duration_seconds) {
        app.refresh();
        meter.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[my_pid]),
            true,
            ProcessRefreshKind::nothing().with_memory().without_tasks(),
        );
        let rss = meter.process(my_pid).map_or(0, |process| process.memory());
        println!("{},{},{},{}",
            started.elapsed().as_secs(),
            app.total_process_count(),
            app.system_stats.scan_duration_ms,
            rss,
        );
        thread::sleep(Duration::from_secs(1));
    }
}
