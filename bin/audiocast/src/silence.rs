//! Auto-stop on silence: watch the capture stream and stop the cast once the
//! input has been quiet for a configured stretch (e.g. the record ran out).
//!
//! Detection is RMS over one-second windows, so brief end-groove pops don't
//! reset the clock the way a peak detector would, while any program material
//! (well above a turntable's noise floor) does.

use std::sync::Arc;
use std::time::Duration;

use application::StopCasting;
use domain::{CastController, CastSession, CastState};
use infrastructure::PcmSource;
use tokio::sync::{broadcast, Mutex};

const WINDOW: Duration = Duration::from_secs(1);

/// Subscribe to the PCM broadcast and stop the active cast after `timeout` of
/// continuous silence (RMS below `threshold`, as a fraction of full scale).
pub fn spawn(
    audio: Arc<dyn PcmSource>,
    cast: Arc<dyn CastController>,
    session: Arc<Mutex<CastSession>>,
    threshold: f32,
    timeout: Duration,
) {
    let format = audio.format();
    let mut rx = audio.subscribe();
    let mut tracker = SilenceTracker::new(threshold, format.sample_rate, format.channels);

    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(chunk) => {
                    let Some(silent) = tracker.feed(&chunk) else {
                        continue;
                    };
                    let mut session = session.lock().await;
                    // The clock only runs during playback; otherwise silence
                    // accrued before the user hits play (empty platter) would
                    // stop a fresh cast almost immediately.
                    if session.state() != CastState::Playing {
                        tracker.reset();
                        continue;
                    }
                    if silent < timeout {
                        continue;
                    }
                    tracing::info!(
                        silent_secs = silent.as_secs(),
                        "input silent; auto-stopping cast"
                    );
                    if let Err(e) = StopCasting::new(cast.clone()).execute(&mut session).await {
                        tracing::warn!(error = %e, "auto-stop failed; will retry");
                    }
                    // Restart the clock either way so a retry waits a full
                    // timeout instead of re-firing every window.
                    tracker.reset();
                }
                // Lagging just skipped some audio; a gap can only make us
                // *less* eager to stop, never spuriously silent.
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// Pure silence accounting: feed interleaved i16-LE PCM, and at each completed
/// window learn how long the input has been continuously below the threshold.
struct SilenceTracker {
    threshold: f64,
    samples_per_window: u64,
    sum_squares: f64,
    samples: u64,
    silent: Duration,
}

impl SilenceTracker {
    fn new(threshold: f32, sample_rate: u32, channels: u16) -> Self {
        Self {
            threshold: threshold as f64,
            samples_per_window: u64::from(sample_rate) * u64::from(channels.max(1)),
            sum_squares: 0.0,
            samples: 0,
            silent: Duration::ZERO,
        }
    }

    /// Accumulate a PCM chunk. Returns the running silence duration each time
    /// a window completes, `None` while one is still filling.
    fn feed(&mut self, pcm: &[u8]) -> Option<Duration> {
        for sample in pcm.chunks_exact(2) {
            let s = f64::from(i16::from_le_bytes([sample[0], sample[1]])) / 32_768.0;
            self.sum_squares += s * s;
        }
        self.samples += (pcm.len() / 2) as u64;
        if self.samples < self.samples_per_window {
            return None;
        }

        let rms = (self.sum_squares / self.samples as f64).sqrt();
        // Chunks rarely align with the window boundary; credit the actual
        // audio time this window covered.
        let elapsed = WINDOW.mul_f64(self.samples as f64 / self.samples_per_window as f64);
        self.sum_squares = 0.0;
        self.samples = 0;

        if rms < self.threshold {
            self.silent += elapsed;
        } else {
            self.silent = Duration::ZERO;
        }
        Some(self.silent)
    }

    fn reset(&mut self) {
        self.silent = Duration::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 1_000; // small window so tests stay tiny
    const CHANNELS: u16 = 1;

    fn pcm(amplitude: i16, samples: usize) -> Vec<u8> {
        std::iter::repeat_with(|| amplitude.to_le_bytes())
            .take(samples)
            .flatten()
            .collect()
    }

    fn window(tracker: &mut SilenceTracker, amplitude: i16) -> Duration {
        tracker
            .feed(&pcm(amplitude, RATE as usize))
            .expect("full window must complete")
    }

    #[test]
    fn silence_accumulates_across_windows() {
        let mut t = SilenceTracker::new(0.01, RATE, CHANNELS);
        assert_eq!(window(&mut t, 0), Duration::from_secs(1));
        assert_eq!(window(&mut t, 100), Duration::from_secs(2)); // below threshold
    }

    #[test]
    fn loud_window_resets_the_clock() {
        let mut t = SilenceTracker::new(0.01, RATE, CHANNELS);
        window(&mut t, 0);
        assert_eq!(window(&mut t, 10_000), Duration::ZERO);
        assert_eq!(window(&mut t, 0), Duration::from_secs(1));
    }

    #[test]
    fn partial_chunks_fill_a_window() {
        let mut t = SilenceTracker::new(0.01, RATE, CHANNELS);
        let half = pcm(0, RATE as usize / 2);
        assert_eq!(t.feed(&half), None);
        assert_eq!(t.feed(&half), Some(Duration::from_secs(1)));
    }

    #[test]
    fn reset_clears_accumulated_silence() {
        let mut t = SilenceTracker::new(0.01, RATE, CHANNELS);
        window(&mut t, 0);
        t.reset();
        assert_eq!(window(&mut t, 0), Duration::from_secs(1));
    }
}
