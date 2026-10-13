//! Application state: everything the UI needs to draw a frame.

use crate::bpm::{Status, TapTracker};

/// How long a tap keeps the "beat" marker lit, in milliseconds.
pub const PULSE_MS: f64 = 130.0;

/// After this long without a tap the reading is reported as idle.
pub const IDLE_AFTER_MS: u128 = 3_000;

/// Number of intervals kept for the history graph.
///
/// This is a rolling window: once it is full the oldest interval is dropped on
/// every tap, so the graph keeps scrolling instead of freezing. The graph only
/// draws as many bars as the terminal is wide, so 20 is already more than fits
/// on most windows while still giving a useful "how steady am I" picture.
pub const HISTORY_LEN: usize = 20;

pub struct App {
    pub tracker: TapTracker,
    /// Monotonic session clock in milliseconds.
    ///
    /// Kept explicitly instead of derived from `Instant` so that tests (and the
    /// headless render demo) can drive time by hand; the caller feeds real
    /// frame deltas into [`App::tick`].
    elapsed_ms: u128,
    /// Decaying "just tapped" animation value in `0.0..=1.0`.
    pub pulse: f64,
    /// Taps the user performed this session.
    pub total_taps: u64,
    /// Short-lived status line, e.g. "Reset.".
    pub message: Option<&'static str>,
    /// Recent intervals in ms, oldest first, for the history graph.
    pub history: Vec<f64>,
    /// Set when the process should exit.
    pub should_quit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            tracker: TapTracker::new(),
            elapsed_ms: 0,
            pulse: 0.0,
            total_taps: 0,
            message: None,
            history: Vec::with_capacity(HISTORY_LEN),
            should_quit: false,
        }
    }

    /// Milliseconds since the session started.
    pub fn now_ms(&self) -> u128 {
        self.elapsed_ms
    }

    /// Install one virtual tap `interval_ms` in the past and start the session
    /// clock at that offset.
    ///
    /// This gives the very first real tap a sensible reading instead of
    /// starting from "no data". The seed is deliberately not counted as a tap.
    pub fn seed(&mut self, interval_ms: u128) {
        self.elapsed_ms = interval_ms;
        self.tracker.tap(0);
        self.pulse = 1.0;
    }

    /// Register one space-bar tap.
    pub fn tap(&mut self) {
        // The tracker decides whether this tap counts and hands back the
        // interval it completed. Comparing queue lengths here would be wrong:
        // the tracker's queue is capped, so its length stops changing once full.
        let interval = self.tracker.tap(self.elapsed_ms);
        self.pulse = 1.0;
        self.total_taps += 1;
        if let Some(interval) = interval {
            self.history.push(interval);
            if self.history.len() > HISTORY_LEN {
                self.history.remove(0);
            }
        }
        self.message = None;
    }

    /// Clear the current measurement.
    pub fn reset(&mut self, message: &'static str) {
        self.tracker.clear();
        self.pulse = 0.0;
        self.total_taps = 0;
        self.history.clear();
        self.message = Some(message);
    }

    /// Advance the session clock and decay the pulse animation.
    ///
    /// `real_delta_ms` is how much wall-clock time passed since the previous
    /// frame; the session clock only ever moves forward.
    pub fn tick(&mut self, real_delta_ms: f64) {
        self.elapsed_ms += real_delta_ms.max(0.0).round() as u128;
        if self.pulse > 0.0 {
            self.pulse = (self.pulse - real_delta_ms / PULSE_MS).max(0.0);
        }
    }

    /// Current tempo estimate in BPM, if there is one.
    pub fn bpm(&self) -> Option<f64> {
        self.tracker.bpm()
    }

    /// Measuring / tapping / idle, as of right now.
    pub fn status(&self) -> Status {
        self.tracker.status(self.elapsed_ms, IDLE_AFTER_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeding_gives_the_first_tap_a_reading_without_counting_a_tap() {
        let mut app = App::new();
        app.seed(500);
        // One virtual tap produces no interval on its own...
        assert_eq!(app.bpm(), None);
        assert_eq!(app.total_taps, 0);
        // ...but the first real tap completes it and reads 120 BPM.
        app.tap();
        assert_eq!(app.total_taps, 1);
        let bpm = app.bpm().unwrap();
        assert!((bpm - 120.0).abs() < 1e-9, "got {bpm}");
    }

    #[test]
    fn tapping_on_the_beat_tracks_the_tempo() {
        let mut app = App::new();
        app.seed(500);
        app.tap(); // completes the seeded interval
        for _ in 0..4 {
            app.tick(500.0);
            app.tap();
        }
        assert_eq!(app.total_taps, 5);
        let bpm = app.bpm().unwrap();
        assert!((bpm - 120.0).abs() < 1e-9, "got {bpm}");
        assert_eq!(app.history.len(), 5);
    }

    #[test]
    fn reset_clears_everything() {
        let mut app = App::new();
        app.seed(500);
        app.tap();
        assert!(app.bpm().is_some());
        app.reset("x");
        assert_eq!(app.bpm(), None);
        assert_eq!(app.total_taps, 0);
        assert!(app.history.is_empty());
        assert_eq!(app.message, Some("x"));
    }

    #[test]
    fn a_tap_arriving_too_quickly_is_not_counted_as_an_interval() {
        let mut app = App::new();
        app.seed(500);
        app.tap(); // 500 ms after the virtual tap: a valid beat
        app.tap(); // immediately after: noise, not counted
        assert_eq!(app.total_taps, 2);
        assert_eq!(app.history.len(), 1);
        // Still the 120 BPM estimate, undamaged by the double-tap.
        let bpm = app.bpm().unwrap();
        assert!((bpm - 120.0).abs() < 1e-9, "got {bpm}");
    }

    #[test]
    fn pulse_decays_and_clamps_at_zero() {
        let mut app = App::new();
        app.tap();
        assert_eq!(app.pulse, 1.0);
        app.tick(65.0);
        assert!((app.pulse - 0.5).abs() < 1e-6, "got {}", app.pulse);
        app.tick(10_000.0);
        assert_eq!(app.pulse, 0.0);
    }

    #[test]
    fn a_single_tap_is_still_measuring() {
        let mut app = App::new();
        app.tap();
        assert_eq!(app.status(), Status::Measuring);
    }

    #[test]
    fn the_reading_keeps_tracking_after_the_windows_are_full() {
        // Regression: history used to be appended based on the tracker's queue
        // length, which stops changing once that queue is full. The graph then
        // froze and, because the tracker also feeds the BPM average, the reading
        // stuck at whatever it was when the queue filled.
        let mut app = App::new();
        app.seed(500);
        app.tap();

        // Long enough to fill both the tracker window (12) and the graph (20).
        for _ in 0..300 {
            app.tick(500.0);
            app.tap();
        }
        let slow = app.bpm().expect("reading");
        assert!(
            (slow - 120.0).abs() < 1e-6,
            "steady 500 ms taps: got {slow}"
        );
        assert_eq!(app.history.len(), HISTORY_LEN);

        // Double the tempo: the reading must follow, not stay stuck.
        for _ in 0..100 {
            app.tick(250.0);
            app.tap();
        }
        let fast = app.bpm().expect("reading");
        assert!((fast - 240.0).abs() < 1e-6, "after speeding up: got {fast}");

        // And the history must hold the newest intervals, not old ones.
        assert!(
            app.history.iter().all(|v| (*v - 250.0).abs() < 1e-6),
            "history still held stale intervals: {:?}",
            app.history
        );
    }

    #[test]
    fn status_goes_idle_after_the_threshold() {
        let mut app = App::new();
        app.seed(500);
        app.tick(500.0);
        app.tap();
        assert_eq!(app.status(), Status::Tapping);
        app.tick(IDLE_AFTER_MS as f64 + 1.0);
        assert_eq!(app.status(), Status::Idle);
    }
}
