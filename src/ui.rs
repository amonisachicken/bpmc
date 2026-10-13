//! Rendering: turns [`App`] state into terminal frames.

use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::bpm::{self, Status};
use crate::i18n::Lang;

/// Digits drawn on a 5x3 grid, one string per row.
const DIGITS: [[&str; 5]; 10] = [
    ["███", "█ █", "█ █", "█ █", "███"],
    ["  █", "  █", "  █", "  █", "  █"],
    ["███", "  █", "███", "█  ", "███"],
    ["███", "  █", "███", "  █", "███"],
    ["█ █", "█ █", "███", "  █", "  █"],
    ["███", "█  ", "███", "  █", "███"],
    ["███", "█  ", "███", "█ █", "███"],
    ["███", "  █", "  █", "  █", "  █"],
    ["███", "█ █", "███", "█ █", "███"],
    ["███", "█ █", "███", "  █", "███"],
];

/// A decimal point, aligned with the bottom row of the digits.
const DOT: [&str; 5] = ["   ", "   ", "   ", "   ", " █ "];

/// Palette shared by the whole UI.
struct Theme {
    accent: Color,
    dim: Color,
    ok: Color,
    warn: Color,
}

const THEME: Theme = Theme {
    accent: Color::Rgb(0, 200, 255),
    dim: Color::Rgb(110, 120, 135),
    ok: Color::Rgb(120, 230, 140),
    warn: Color::Rgb(255, 190, 70),
};

/// Draw one frame.
pub fn draw(frame: &mut Frame, app: &App, lang: Lang) {
    let t = lang.text();
    let area = frame.area();

    // A tiny terminal cannot show the dashboard; keep the center readable.
    if area.width < 24 || area.height < 6 {
        let line = Paragraph::new(t.terminal_too_small)
            .style(Style::default().fg(THEME.warn))
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });
        frame.render_widget(line, center_vertically(area, 1));
        return;
    }

    let show_stats = area.height >= 10;
    let show_history = area.height >= 14;

    let rows = Layout::vertical([
        Constraint::Length(2),                                // title + status
        Constraint::Min(3),                                   // the number
        Constraint::Length(if show_stats { 4 } else { 0 }),   // stats
        Constraint::Length(if show_history { 7 } else { 0 }), // history
        Constraint::Length(2),                                // hints
    ])
    .split(area);

    render_title(frame, rows[0], app, lang);
    render_bpm(frame, rows[1], app, lang);
    if show_stats {
        render_stats(frame, rows[2], app, lang);
    }
    if show_history {
        render_history(frame, rows[3], app, lang);
    }
    render_hints(frame, rows[4], lang);
}

/// Center a fixed-height box inside `area`.
fn center_vertically(area: Rect, height: u16) -> Rect {
    let [box_area] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    box_area
}

fn render_title(frame: &mut Frame, area: Rect, app: &App, lang: Lang) {
    let t = lang.text();
    let (status, color) = match app.status() {
        Status::Measuring => (t.measuring, THEME.dim),
        Status::Tapping => (t.hint_short, THEME.ok),
        Status::Idle => (t.idle, THEME.warn),
    };
    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", t.title),
            Style::default()
                .fg(THEME.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("· ", Style::default().fg(THEME.dim)),
        Span::styled(status, Style::default().fg(color)),
    ]);
    frame.render_widget(
        Paragraph::new(line).alignment(Alignment::Center),
        center_vertically(area, 1),
    );
}

fn render_bpm(frame: &mut Frame, area: Rect, app: &App, lang: Lang) {
    let t = lang.text();
    let status = app.status();
    let color = if status == Status::Idle {
        THEME.dim
    } else {
        THEME.accent
    };
    let pulse_on = app.pulse > 0.5;

    // Preferred: large block digits, when they fit the area.
    if let Some(rows) = big_digits(app.bpm()) {
        let width = rows.first().map(|r| r.chars().count()).unwrap_or(0) as u16;
        if width + 4 <= area.width && area.height >= 5 {
            let lines: Vec<Line> = rows
                .into_iter()
                .map(|row| Line::from(Span::styled(row, Style::default().fg(color))))
                .collect();
            frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
            return;
        }
    }

    // Fallback: one inline line, centered.
    let number = match app.bpm() {
        Some(bpm) => bpm::format_bpm(bpm),
        None => "--".to_string(),
    };
    let line = Line::from(vec![
        Span::styled(
            format!(" {number} "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("{} ", t.bpm_unit), Style::default().fg(THEME.dim)),
        Span::styled(
            if pulse_on { "●" } else { "○" },
            Style::default().fg(THEME.accent),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(line).alignment(Alignment::Center),
        center_vertically(area, 1),
    );
}

/// Build the big-digit art for `bpm`; `None` when there is no reading yet.
fn big_digits(bpm: Option<f64>) -> Option<Vec<String>> {
    let text = bpm::format_bpm(bpm?);
    let mut rows = vec![String::new(); 5];
    for (i, ch) in text.chars().enumerate() {
        let glyph: [&str; 5] = match ch {
            '0'..='9' => DIGITS[ch as usize - '0' as usize],
            '.' => DOT,
            _ => continue,
        };
        for (row, g) in rows.iter_mut().zip(glyph.iter()) {
            if i > 0 {
                row.push(' ');
            }
            row.push_str(g);
        }
    }
    Some(rows)
}

fn render_stats(frame: &mut Frame, area: Rect, app: &App, lang: Lang) {
    let t = lang.text();
    let last = app
        .tracker
        .last_interval()
        .map(|ms| format!("{ms:.0} ms"))
        .unwrap_or_else(|| t.none.to_string());
    let jitter = app
        .tracker
        .interval_std_dev()
        .map(|ms| format!("±{ms:.0} ms"))
        .unwrap_or_else(|| t.none.to_string());

    let columns = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .split(area);

    let stats = [
        (t.taps, app.total_taps.to_string(), THEME.accent),
        (t.last_interval, last, THEME.ok),
        (t.steady, jitter, THEME.warn),
    ];

    for (col, (label, value, color)) in columns.iter().zip(stats) {
        let text = Text::from(vec![
            Line::from(Span::styled(label, Style::default().fg(THEME.dim))),
            Line::from(Span::styled(
                value,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
        ]);
        frame.render_widget(Paragraph::new(text).alignment(Alignment::Center), *col);
    }

    if let Some(msg) = app.message {
        let row = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Span::styled(msg, Style::default().fg(THEME.dim)))
                .alignment(Alignment::Center),
            row,
        );
    }
}

/// Bar graph of the most recent intervals.
fn render_history(frame: &mut Frame, area: Rect, app: &App, lang: Lang) {
    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    let t = lang.text();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(THEME.dim))
        .padding(Padding::horizontal(1))
        .title(Span::styled(
            format!(" {} ", t.history),
            Style::default().fg(THEME.dim),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    // Keep the newest bars, drawn left to right, oldest first.
    let capacity = inner.width as usize;
    let skip = app.history.len().saturating_sub(capacity);
    let history: Vec<f64> = app.history.iter().skip(skip).copied().collect();

    if history.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(t.measuring, Style::default().fg(THEME.dim)))
                .alignment(Alignment::Center),
            center_vertically(inner, 1),
        );
        return;
    }

    let max = history.iter().copied().fold(f64::MIN, f64::max).max(1.0);
    let height = inner.height as usize;

    let mut lines: Vec<Line> = Vec::with_capacity(height);
    for row in 0..height {
        // `from_bottom` counts rows up from the bottom of the graph.
        let from_bottom = height - row;
        let mut spans: Vec<Span> = Vec::with_capacity(history.len());
        for &value in &history {
            let scaled = value / max * height as f64;
            let ch = if scaled >= from_bottom as f64 {
                '█'
            } else if scaled > (from_bottom - 1) as f64 {
                let frac = scaled - (from_bottom - 1) as f64;
                BARS[((frac * BARS.len() as f64) as usize).min(BARS.len() - 1)]
            } else {
                ' '
            };
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(THEME.accent),
            ));
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

fn render_hints(frame: &mut Frame, area: Rect, lang: Lang) {
    let t = lang.text();
    let items = [
        (t.hint_tap, THEME.accent),
        (t.hint_quit, THEME.dim),
        (t.hint_reset, THEME.dim),
    ];
    let mut spans = Vec::new();
    for (i, (label, color)) in items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", Style::default()));
        }
        spans.push(Span::styled(*label, Style::default().fg(*color)));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        center_vertically(area, 1),
    );
}
