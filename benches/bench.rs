use std::time::Instant;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

fn main() {
    let mut sys = System::new_all();
    println!("All processes count: {}", sys.processes().len());

    std::thread::sleep(std::time::Duration::from_millis(200));

    // Heavy
    let start = Instant::now();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );
    println!("heavy (everything):       {:>8?}", start.elapsed());

    // Light
    let start = Instant::now();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cpu().with_memory(),
    );
    println!("light (cpu+mem):          {:>8?}", start.elapsed());

    // Redundant system stats refresh
    let start = Instant::now();
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    println!("redundant cpu+mem:        {:>8?}", start.elapsed());
}
