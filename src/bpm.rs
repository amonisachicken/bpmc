//! BPM math: turning a stream of tap timestamps into a tempo estimate.

use std::collections::VecDeque;

/// How many of the most recent taps participate in the estimate.
pub const WINDOW: usize = 12;

/// A tap older than this (in ms) is not paired with the next tap, because the
/// user almost certainly paused; counting that gap would report a nonsense BPM.
pub const MAX_INTERVAL_MS: f64 = 3_000.0;

/// Taps closer together than this are treated as one gesture. Key autorepeat,
/// a double-fired key event or a testing harness can otherwise produce a
/// zero-length interval, and `60000 / 0` is not a tempo.
pub const MIN_INTERVAL_MS: f64 = 20.0;

/// A freshly tapped session with no data yet reads as "measuring".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Fewer than two usable taps: no interval exists yet.
    Measuring,
    /// Tapping is ongoing (or paused only briefly).
    Tapping,
    /// No tap for a while: the number is stale.
    Idle,
}

/// Rolling tempo estimator over the last [`WINDOW`] taps.
#[derive(Debug, Clone)]
pub struct TapTracker {
    /// Timestamps in milliseconds, oldest first, capped at [`WINDOW`].
    taps: VecDeque<u128>,
    /// Intervals (ms) between consecutive *counted* taps, capped at [`WINDOW`].
    intervals: VecDeque<f64>,
    /// Taps dropped because the preceding gap was longer than [`MAX_INTERVAL_MS`].
    resets: u64,
}

impl Default for TapTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TapTracker {
    pub fn new() -> Self {
        Self {
            taps: VecDeque::with_capacity(WINDOW),
            intervals: VecDeque::with_capacity(WINDOW),
            resets: 0,
        }
    }

    /// Forget all data (the reset key).
    pub fn clear(&mut self) {
        self.taps.clear();
        self.intervals.clear();
        self.resets = 0;
    }

    /// Record one tap at `now_ms` (a monotonic timestamp in milliseconds).
    pub fn tap(&mut self, now_ms: u128) {
        if let Some(&last) = self.taps.back() {
            let delta = now_ms.saturating_sub(last) as f64;
            if delta > MAX_INTERVAL_MS {
                // Long pause: restart the measurement from this tap.
                self.taps.clear();
                self.intervals.clear();
                self.resets += 1;
            } else if delta >= MIN_INTERVAL_MS {
                self.intervals.push_back(delta);
                while self.intervals.len() > WINDOW {
                    self.intervals.pop_front();
                }
            } else {
                // Too fast to be a deliberate tap: ignore it entirely rather
                // than letting it dominate the average.
                return;
            }
        }
        self.taps.push_back(now_ms);
        while self.taps.len() > WINDOW {
            self.taps.pop_front();
        }
    }

    /// Total number of taps recorded in this session (including ones that
    /// restarted the measurement).
    pub fn taps_in_window(&self) -> usize {
        self.taps.len()
    }

    /// Intervals currently feeding the estimate, oldest first.
    pub fn intervals(&self) -> impl Iterator<Item = f64> + '_ {
        self.intervals.iter().copied()
    }

    /// Most recent interval in ms, if any.
    pub fn last_interval(&self) -> Option<f64> {
        self.intervals.back().copied()
    }

    /// Milliseconds since the most recent tap, or `None` when there is none.
    pub fn since_last_tap(&self, now_ms: u128) -> Option<u128> {
        self.taps.back().map(|&t| now_ms.saturating_sub(t))
    }

    /// Mean interval over the window, in ms.
    pub fn mean_interval(&self) -> Option<f64> {
        if self.intervals.is_empty() {
            return None;
        }
        let sum: f64 = self.intervals.iter().sum();
        Some(sum / self.intervals.len() as f64)
    }

    /// Standard deviation of the intervals, in ms: the "steadiness" of tapping.
    pub fn interval_std_dev(&self) -> Option<f64> {
        let mean = self.mean_interval()?;
        if self.intervals.len() < 2 {
            return Some(0.0);
        }
        let variance = self
            .intervals
            .iter()
            .map(|d| (d - mean) * (d - mean))
            .sum::<f64>()
            / self.intervals.len() as f64;
        Some(variance.sqrt())
    }

    /// Current estimate in beats per minute.
    ///
    /// `None` means "not enough data yet". The estimate is the mean of the
    /// rolling intervals, which is more stable than timing only the last pair
    /// while still reacting quickly to a tempo change.
    pub fn bpm(&self) -> Option<f64> {
        let mean = self.mean_interval()?;
        if mean <= 0.0 {
            return None;
        }
        Some(60_000.0 / mean)
    }

    /// Classify the tracker given the current time.
    pub fn status(&self, now_ms: u128, idle_after_ms: u128) -> Status {
        if self.intervals.is_empty() {
            return Status::Measuring;
        }
        match self.since_last_tap(now_ms) {
            Some(since) if since > idle_after_ms => Status::Idle,
            _ => Status::Tapping,
        }
    }
}

/// Render a BPM value with the precision that is actually meaningful.
///
/// Below 100 BPM one decimal is readable; above it the decimal digit would
/// jitter on every tap, so show whole beats.
pub fn format_bpm(bpm: f64) -> String {
    if bpm < 100.0 {
        format!("{bpm:.1}")
    } else {
        format!("{bpm:.0}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_taps_means_no_estimate() {
        let t = TapTracker::new();
        assert_eq!(t.bpm(), None);
        assert_eq!(t.status(0, 1_000), Status::Measuring);
    }

    #[test]
    fn one_tap_still_has_no_interval() {
        let mut t = TapTracker::new();
        t.tap(1_000);
        assert_eq!(t.bpm(), None);
        assert_eq!(t.taps_in_window(), 1);
    }

    #[test]
    fn steady_500ms_taps_are_120_bpm() {
        let mut t = TapTracker::new();
        for i in 0..5 {
            t.tap(i * 500);
        }
        let bpm = t.bpm().unwrap();
        assert!((bpm - 120.0).abs() < 1e-9, "got {bpm}");
    }

    #[test]
    fn long_pause_restarts_the_measurement() {
        let mut t = TapTracker::new();
        t.tap(0);
        t.tap(500);
        t.tap(500 + 10_000); // pause > MAX_INTERVAL_MS
        assert_eq!(t.intervals().count(), 0);
        assert_eq!(t.bpm(), None);
        assert_eq!(t.resets, 1);
    }

    #[test]
    fn window_is_bounded() {
        let mut t = TapTracker::new();
        for i in 0..(WINDOW as u128 * 3) {
            t.tap(i * 500);
        }
        assert_eq!(t.taps_in_window(), WINDOW);
        assert_eq!(t.intervals().count(), WINDOW);
    }

    #[test]
    fn idle_status_after_the_threshold() {
        let mut t = TapTracker::new();
        t.tap(0);
        t.tap(500);
        assert_eq!(t.status(600, 2_000), Status::Tapping);
        assert_eq!(t.status(3_000, 2_000), Status::Idle);
    }

    #[test]
    fn std_dev_is_zero_for_even_taps() {
        let mut t = TapTracker::new();
        for i in 0..4 {
            t.tap(i * 600);
        }
        assert!(t.interval_std_dev().unwrap().abs() < 1e-9);
    }

    #[test]
    fn formatting_switches_precision_at_100() {
        assert_eq!(format_bpm(99.94), "99.9");
        assert_eq!(format_bpm(120.4), "120");
    }

    #[test]
    fn near_simultaneous_taps_are_ignored() {
        let mut t = TapTracker::new();
        t.tap(1_000);
        t.tap(1_000); // duplicate event
        t.tap(1_005); // key autorepeat
        assert_eq!(t.intervals().count(), 0);
        assert_eq!(t.bpm(), None);
        // The real tap after the noise still measures normally.
        t.tap(1_500);
        let bpm = t.bpm().unwrap();
        assert!((bpm - 120.0).abs() < 1e-9, "got {bpm}");
    }

    #[test]
    fn very_fast_tapping_below_the_floor_is_rejected() {
        let mut t = TapTracker::new();
        // 1 ms apart is not a human tempo.
        for i in 0..5 {
            t.tap(1_000 + i);
        }
        let seen: Vec<f64> = t.intervals().collect();
        assert!(
            seen.is_empty(),
            "floor is {MIN_INTERVAL_MS}, intervals seen: {seen:?}"
        );
        assert_eq!(t.bpm(), None);

        // Rejected taps are not kept as an anchor: the next real beat is
        // measured from the last accepted tap.
        let mut t = TapTracker::new();
        t.tap(1_000);
        t.tap(1_001); // rejected
        t.tap(1_002); // rejected
        t.tap(1_510); // real beat, 510 ms after the last accepted tap
        let bpm = t.bpm().unwrap();
        assert!((bpm - 117.647).abs() < 0.01, "got {bpm}");
    }

    #[test]
    fn the_fastest_plausible_tempo_still_works() {
        let mut t = TapTracker::new();
        // 240 BPM = 250 ms between beats.
        t.tap(0);
        t.tap(250);
        let bpm = t.bpm().unwrap();
        assert!((bpm - 240.0).abs() < 1e-9, "got {bpm}");
    }
}
