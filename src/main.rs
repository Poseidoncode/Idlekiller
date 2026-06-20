use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{error::Error, io, time::{Duration, Instant}};

use idlekiller::app::App;
use idlekiller::ui;

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(1000);
    let mut app = App::new();
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
    app.refresh();

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if app.is_searching {
                        match key.code {
                            KeyCode::Enter => {
                                app.is_searching = false;
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
                            }
                            KeyCode::Char('K') => {
                                app.kill_all_wasteful();
                            }
                            _ => {}
                        }
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
                    event::MouseEventKind::Down(event::MouseButton::Left) => {
                        if let Ok(size) = crossterm::terminal::size() {
                            app.handle_click(mouse.column, mouse.row, size.0);
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
