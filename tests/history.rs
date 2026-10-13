//! Rendering tests for the interval history graph.
//!
//! These cover the user-visible behaviour of the graph rather than the raw
//! buffer: after many taps the newest interval must keep showing up, and the
//! oldest bars must be dropped instead of the graph freezing.

use bpmc::app::{App, HISTORY_LEN};
use bpmc::i18n::Lang;
use bpmc::ui;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

fn rendered_text(app: &App) -> String {
    let backend = TestBackend::new(WIDTH, HEIGHT);
    let mut terminal = Terminal::new(backend).expect("test backend");
    terminal
        .draw(|frame| ui::draw(frame, app, Lang::En))
        .expect("draw");
    let buffer = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let symbol = buffer[(x, y)].symbol();
            out.push_str(if symbol.is_empty() { " " } else { symbol });
        }
        out.push('\n');
    }
    out
}

/// The rows of the history box only (it is the second-to-last block).
fn history_rows(text: &str) -> String {
    text.lines()
        .filter(|line| line.contains('█') || line.contains('▅') || line.contains('▁'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn history_keeps_updating_after_many_taps() {
    let mut app = App::new();
    app.seed(500);

    // Alternate between two tempos so every new bar is visually different.
    let mut previous = String::new();
    let mut changes = 0;
    let mut frozen_at: Option<usize> = None;

    for i in 0..(HISTORY_LEN * 4) {
        let delta = if i % 2 == 0 { 400.0 } else { 640.0 };
        app.tick(delta);
        app.tap();
        app.tick(1_000.0); // let the pulse decay so only the graph differs

        if app.history.len() > 3 {
            let text = history_rows(&rendered_text(&app));
            if text == previous {
                changes += 1;
                frozen_at.get_or_insert(i);
            }
            previous = text;
        }
    }

    // With an alternating tempo the graph must differ on every single tap.
    assert_eq!(
        changes, 0,
        "graph stopped changing after tap {:?} (HISTORY_LEN={HISTORY_LEN})",
        frozen_at
    );
}

#[test]
fn history_is_trimmed_to_the_cap() {
    let mut app = App::new();
    app.seed(500);
    for _ in 0..(HISTORY_LEN * 3) {
        app.tick(500.0);
        app.tap();
    }
    assert_eq!(
        app.history.len(),
        HISTORY_LEN,
        "history should hold exactly the newest {HISTORY_LEN} intervals"
    );
    assert!(
        app.history.len() <= 20,
        "the graph is meant to be a short rolling window"
    );
}

#[test]
fn oldest_bars_are_dropped_not_kept() {
    let mut app = App::new();
    app.seed(500);
    // Fill the window with a distinctive first value, then push it out.
    app.tick(999.0);
    app.tap();
    for _ in 0..HISTORY_LEN {
        app.tick(300.0);
        app.tap();
    }
    assert_eq!(app.history.len(), HISTORY_LEN);
    assert!(
        !app.history.iter().any(|v| (*v - 999.0).abs() < 1.0),
        "the oldest interval should have been discarded: {:?}",
        app.history
    );
    assert!(
        app.history.iter().all(|v| (*v - 300.0).abs() < 1.0),
        "the window should now contain only the newest intervals: {:?}",
        app.history
    );
}
