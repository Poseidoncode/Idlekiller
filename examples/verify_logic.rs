use idlekiller::app::App;

fn main() {
    let mut app = App::new();
    app.refresh();

    let own_pid = sysinfo::get_current_pid()
        .expect("should get current pid")
        .as_u32();
    assert!(
        app.processes.iter().any(|p| p.pid.as_u32() == own_pid),
        "own process should appear in the process list"
    );

    // apply_filter should keep a non-empty list when no search query
    let baseline_len = app.processes.len();
    assert!(
        !app.processes.is_empty(),
        "filtered process list should not be empty"
    );

    // cycling sort and toggling direction should not panic
    app.cycle_sort_column();
    app.toggle_sort_direction();

    // searching with a non-matching pattern should yield an empty list
    app.search_query = "this_should_not_match_any_real_process".to_string();
    app.apply_filter();
    assert!(
        app.processes.is_empty(),
        "non-matching search should yield empty list"
    );

    // clearing the search should restore the original list
    app.search_query.clear();
    app.apply_filter();
    assert_eq!(
        app.processes.len(),
        baseline_len,
        "clearing search should restore the full list"
    );

    // first press of 'kill all wasteful' should not include our own pid
    app.kill_all_wasteful();
    assert!(
        !app.wasteful_targets.iter().any(|p| p.as_u32() == own_pid),
        "own process should not be considered wasteful"
    );

    println!(
        "IdleKiller logic verification passed ({} processes seen)",
        baseline_len
    );
}
