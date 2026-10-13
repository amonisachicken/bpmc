//! Headless render check: draw the UI into an in-memory buffer and print it.
//!
//! This needs no TTY, so it can be run in CI or piped to a file:
//!
//! ```text
//! cargo run --example render_demo            # print every layout (English)
//! cargo run --example render_demo -- zh      # ... in Chinese
//! cargo run --example render_demo -- --check # smoke test: exit 1 if blank
//! ```
//!
//! It is the fastest way to eyeball the layout without a terminal, and the
//! `--check` mode is what CI runs.

use std::io;

use bpmc::app::{App, HISTORY_LEN};
use bpmc::i18n::Lang;
use bpmc::ui;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render(app: &App, lang: Lang, width: u16, height: u16) -> Terminal<TestBackend> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test backend");
    terminal
        .draw(|frame| ui::draw(frame, app, lang))
        .expect("draw");
    terminal
}

/// Flatten the buffer to text.
///
/// A double-width (CJK) glyph occupies two cells: the glyph plus one empty
/// continuation cell. Rendering that empty cell as a space keeps the layout
/// aligned while making the dump look like the real terminal.
fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            let symbol = buffer[(x, y)].symbol();
            line.push_str(if symbol.is_empty() { " " } else { symbol });
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--check");
    let lang = if args.iter().any(|a| a.eq_ignore_ascii_case("zh")) {
        Lang::Zh
    } else {
        Lang::En
    };
    // A session past the point where the graph window has filled up: the last
    // `HISTORY_LEN` intervals are kept and the oldest ones are dropped, so this
    // is exactly the case that used to freeze. Cycling through four tempos keeps
    // the bars visually distinct, which makes the rolling window obvious.
    let mut app = App::new();
    app.seed(500);
    for i in 0..(HISTORY_LEN * 2) {
        let delta = match i % 4 {
            0 => 460.0,
            1 => 300.0,
            2 => 520.0,
            _ => 380.0,
        };
        app.tick(delta);
        app.tap();
    }
    // The pulse would normally have decayed away by the time we render.
    app.tick(1_000.0);

    let cases: [(u16, u16, &str); 4] = [
        (80, 24, "standard 80x24"),
        (120, 40, "wide 120x40"),
        (50, 16, "compact 50x16"),
        (20, 5, "tiny 20x5"),
    ];

    for (w, h, label) in cases {
        let terminal = render(&app, lang, w, h);
        let text = buffer_text(&terminal);

        if check {
            // Smoke-test mode: assert every layout produced something, and stay
            // silent so CI logs remain readable.
            if text.trim().is_empty() {
                eprintln!("render check failed: empty buffer for {label}");
                std::process::exit(1);
            }
            continue;
        }

        println!("--- {label} ---");
        print!("{text}");
        println!();
    }

    // The idle state is only interesting to look at, not to assert on.
    if !check {
        let mut idle = App::new();
        idle.seed(500);
        idle.tick(500.0);
        idle.tap();
        idle.tick(4_000.0);
        let terminal = render(&idle, lang, 80, 24);
        println!("--- idle 80x24 ---");
        print!("{}", buffer_text(&terminal));
    } else {
        println!(
            "render check ok: {} layout(s) rendered ({})",
            cases.len(),
            if lang == Lang::Zh { "zh" } else { "en" }
        );
    }

    Ok(())
}
