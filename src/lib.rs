//! `bpmc` — a tiny cross-platform TUI that measures your tempo.
//!
//! The crate is split into small, testable pieces:
//!
//! * [`bpm`] — turning tap timestamps into a BPM estimate (pure logic).
//! * [`app`] — session state: pulse animation, tap count, interval history.
//! * [`i18n`] — the Chinese / English string tables and locale detection.
//! * [`ui`] — all terminal drawing.
//!
//! The binary in `src/main.rs` only wires these together and owns the terminal.

pub mod app;
pub mod bpm;
pub mod i18n;
pub mod ui;
