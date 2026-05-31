use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use domain::{AudioFormat, AudioSource};
use tokio::sync::broadcast;

use crate::pcm_source::PcmSource;

const FRAMES_PER_CHUNK: usize = 1024;
const TONE_HZ: f32 = 440.0;
const AMPLITUDE: f32 = 0.2;

/// A continuously-running source that emits a 440 Hz sine tone as interleaved
/// i16 PCM, paced in real time. Stands in for the cpal USB capture (Chunk D) so
/// the `/audio-stream` path can be exercised end-to-end on a dev machine —
/// playable in VLC or a browser.
///
/// It mirrors the real design: capture runs once in the background and feeds a
/// lossy `broadcast` channel; each HTTP listener subscribes to the live edge,
/// so a slow client drops frames rather than stalling the producer.
pub struct SyntheticAudioStream {
    tx: broadcast::Sender<Bytes>,
    format: AudioFormat,
}

impl SyntheticAudioStream {
    /// Start the generator task and return a handle. The task runs for the
    /// lifetime of the process.
    pub fn start(format: AudioFormat) -> Arc<Self> {
        let (tx, _rx) = broadcast::channel(64);
        let handle = Arc::new(Self {
            tx: tx.clone(),
            format,
        });
        tokio::spawn(generate(tx, format));
        handle
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Bytes> {
        self.tx.subscribe()
    }

    pub fn format(&self) -> AudioFormat {
        self.format
    }
}

impl AudioSource for SyntheticAudioStream {
    fn format(&self) -> AudioFormat {
        self.format
    }
}

impl PcmSource for SyntheticAudioStream {
    fn format(&self) -> AudioFormat {
        self.format
    }

    fn subscribe(&self) -> broadcast::Receiver<Bytes> {
        self.tx.subscribe()
    }
}

async fn generate(tx: broadcast::Sender<Bytes>, format: AudioFormat) {
    let chunk_dur =
        Duration::from_secs_f64(FRAMES_PER_CHUNK as f64 / format.sample_rate as f64);
    let mut ticker = tokio::time::interval(chunk_dur);
    let step = 2.0 * PI * TONE_HZ / format.sample_rate as f32;
    let mut phase = 0.0f32;

    loop {
        ticker.tick().await;
        let mut buf = Vec::with_capacity(FRAMES_PER_CHUNK * format.channels as usize * 2);
        for _ in 0..FRAMES_PER_CHUNK {
            let sample = (phase.sin() * AMPLITUDE * i16::MAX as f32) as i16;
            phase += step;
            if phase > 2.0 * PI {
                phase -= 2.0 * PI;
            }
            let le = sample.to_le_bytes();
            for _ in 0..format.channels {
                buf.extend_from_slice(&le);
            }
        }
        // Err means no current subscribers; that's fine — keep generating.
        let _ = tx.send(Bytes::from(buf));
    }
}
