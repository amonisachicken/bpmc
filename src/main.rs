//! `bpmc` — a tiny cross-platform TUI that measures your tempo.
//!
//! Tap the space bar in time with a beat and the app shows the resulting BPM
//! in real time, computed from a rolling window of the most recent intervals.
//!
//! * Language: Chinese when the locale contains `zh`, English otherwise.
//! * Keys: `Space` = tap, `r` = reset, `q` / `Esc` / `Ctrl-C` = quit.
//! * Platforms: Windows, Linux, macOS (anywhere crossterm works).

use std::io::{self, IsTerminal};
use std::panic;
use std::time::{Duration, Instant};

use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::Terminal;

use bpmc::app::App;
use bpmc::i18n::Lang;
use bpmc::ui;

/// Input poll timeout: one animation frame at ~60 FPS.
const FRAME: Duration = Duration::from_millis(16);

/// Distance between the virtual starting tap and "now", in ms.
///
/// A single neutral seed tap means the first real tap immediately yields a
/// reading (120 BPM with this value) instead of `--`, and the estimate then
/// converges on the user's actual tempo within a few taps.
const SEED_INTERVAL_MS: u128 = 500;

fn main() -> io::Result<()> {
    // The UI needs a real terminal on both stdin (keys) and stdout (drawing).
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        eprintln!("bpmc: please run this in an interactive terminal (TUI application).");
        std::process::exit(2);
    }

    let lang = Lang::from_env();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    // If anything panics, restore the terminal before the message is printed,
    // otherwise the user is left in raw mode on the alternate screen.
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    let mut app = App::new();
    app.seed(SEED_INTERVAL_MS);

    let result = run(&mut terminal, &mut app, lang);

    // Restore the terminal on every exit path, including errors.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    lang: Lang,
) -> io::Result<()> {
    // Redraw only when something changed: on input, while the pulse animation
    // is running, and after a resize. When the app is idle at 0% pulse this
    // settles into a single redraw per event, so a forgotten window costs no CPU.
    let mut redraw = true;
    let mut last_frame = Instant::now();

    while !app.should_quit {
        if redraw {
            terminal.draw(|frame| ui::draw(frame, app, lang))?;
            redraw = false;
        }

        if event::poll(FRAME)? {
            match event::read()? {
                Event::Key(key) => {
                    handle_key(app, key, lang);
                    redraw = true;
                }
                // The next draw sizes itself from `frame.area()`, so a resize
                // is handled by simply drawing again.
                Event::Resize(_, _) => redraw = true,
                _ => {}
            }
        }

        // Feed the session clock real elapsed time (not the nominal poll
        // timeout) so the idle timer cannot drift.
        let now = Instant::now();
        let delta_ms = now.duration_since(last_frame).as_secs_f64() * 1_000.0;
        last_frame = now;
        app.tick(delta_ms);
        if app.pulse > 0.0 {
            redraw = true;
        }
    }

    // Final frame so the numbers on screen match the final state.
    terminal.draw(|frame| ui::draw(frame, app, lang))?;
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent, lang: Lang) {
    // Only react to presses: Windows reports both press and release events.
    if key.kind == KeyEventKind::Release {
        return;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => app.should_quit = true,
        KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            let message = lang.text().reset_done;
            app.reset(message);
        }
        // Accept both the named key and a literal space, for terminals that
        // report one but not the other.
        KeyCode::Char(' ') | KeyCode::Null => app.tap(),
        _ => {}
    }
}
