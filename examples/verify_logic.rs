//! Non-destructive example: verify that the background collector has delivered
//! a snapshot. No real process is ever signalled by this example.
use idlekiller::app::App;
use std::time::{Duration, Instant};

fn main() {
    let mut app = App::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while app.total_process_count() == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
        app.refresh();
    }
    assert!(app.total_process_count() > 0, "collector did not produce processes");
    let own_pid = sysinfo::get_current_pid().expect("must have PID");
    assert!(
        (0..app.processes.len()).filter_map(|i| app.process_info(i))
            .any(|p| p.pid == own_pid),
        "own process should appear in the process list"
    );
    let count = app.processes.len();
    app.cycle_sort_column();
    app.toggle_sort_direction();
    app.search_query = "this_should_not_match_any_real_process".into();
    app.apply_filter();
    assert!(app.processes.is_empty());
    app.search_query.clear();
    app.apply_filter();
    assert_eq!(app.processes.len(), count);
    println!("Non-destructive verification passed ({count} processes)");
}
