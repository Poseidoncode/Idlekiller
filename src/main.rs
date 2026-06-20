use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{error::Error, io, time::{Duration, Instant}};

use idlekiller::app::App;
use idlekiller::ui;

/// Guard that restores terminal state on panic or early return.
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // ponytail: named constant for tick rate
    const TICK_RATE_MS: u64 = 1000;

    // Create app before terminal setup so the 100ms CPU-delta sleep
    // doesn't block the terminal (avoids unresponsive startup).
    let mut app = App::new();
    let tick_rate = Duration::from_millis(TICK_RATE_MS);

    // setup terminal
    enable_raw_mode()?;
    let _guard = TerminalGuard; // cleans up on any exit path
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Check minimum terminal size
    let size = terminal.size()?;
    if size.width < 80 || size.height < 24 {
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        return Err(format!("Terminal too small: {}x{} (need at least 80x24)", size.width, size.height).into());
    }

    // run app
    let res = run_app(&mut terminal, &mut app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    tick_rate: Duration,
) -> Result<(), Box<dyn Error>>
where
    <B as ratatui::backend::Backend>::Error: 'static,
{
    let mut last_tick = Instant::now();
    // ponytail: exponential backoff on draw failure to avoid busy-spin
    let mut draw_failures: u32 = 0;

    loop {
        let draw_ok = terminal.draw(|f| ui::draw(f, app)).is_ok();
        if draw_ok {
            if app.dirty {
                app.dirty = false;
            }
            draw_failures = 0; // reset on any successful draw
        } else {
            draw_failures = draw_failures.saturating_add(1);
            // Keep dirty=true so next iteration retries
        }
        // Exponential backoff: 50ms × 2^(failures-1), max ~3.2s
        let draw_backoff = if !draw_ok {
            let n = draw_failures.saturating_sub(1).min(6);
            Some(Duration::from_millis(50 * 2u64.pow(n)))
        } else {
            None
        };

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        // When draws fail, use backoff directly (ignoring tick timeout) so we don't busy-spin
        let timeout = draw_backoff.unwrap_or(timeout);

        if crossterm::event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if app.is_searching {
                        match key.code {
                            KeyCode::Enter => {
                                app.is_searching = false;
                                app.dirty = true;
                            }
                            KeyCode::Esc => {
                                app.is_searching = false;
                                app.search_query.clear();
                                app.apply_filter();
                            }
                            KeyCode::Char(c) => {
                                app.search_query.push(c);
                                app.apply_filter();
                            }
                            KeyCode::Backspace => {
                                app.search_query.pop();
                                app.apply_filter();
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                            KeyCode::Down | KeyCode::Char('j') => app.next(),
                            KeyCode::Up | KeyCode::Char('k') => app.previous(),
                            KeyCode::Enter | KeyCode::Char('x') => app.kill_selected(),
                            KeyCode::Char('s') => app.open_search(),
                            KeyCode::Char('f') | KeyCode::Char('/') => {
                                app.is_searching = true;
                                app.dirty = true;
                            }
                            KeyCode::Char('K') => {
                                app.kill_all_wasteful();
                            }
                            KeyCode::Tab => {
                                app.cycle_sort_column();
                            }
                            KeyCode::BackTab => {
                                app.cycle_sort_column_reverse();
                            }
                            KeyCode::Char('r') => {
                                app.toggle_sort_direction();
                            }
                            _ => {
                                app.confirming_kill_all = false; // any other key cancels confirmation
                            }
                        }
                    }
                },
                Event::Resize(w, h) => {
                    app.dirty = true;
                    if w < 80 || h < 24 {
                        app.message = Some(format!("Terminal too small: {}x{} (need 80x24)", w, h));
                        app.message_instant = Some(Instant::now());
                    }
                },
                Event::Mouse(mouse) => match mouse.kind {
                    event::MouseEventKind::ScrollDown => {
                        app.next();
                        app.next();
                        app.next();
                    },
                    event::MouseEventKind::ScrollUp => {
                        app.previous();
                        app.previous();
                        app.previous();
                    },
                    event::MouseEventKind::Down(event::MouseButton::Left) if !app.is_searching => {
                        if let Ok(size) = terminal.size() {
                            ui::handle_header_click(app, mouse.column, mouse.row, size.width);
                        }
                    },
                    _ => {}
                },
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.refresh();
            last_tick = Instant::now();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
