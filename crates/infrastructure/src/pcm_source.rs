use bytes::Bytes;
use domain::AudioFormat;
use tokio::sync::broadcast;

/// A live source of interleaved i16 PCM frames feeding a lossy `broadcast`
/// channel. Both the synthetic tone and the real cpal capture implement this,
/// so the composition root can pick an audio backend behind a single
/// `Arc<dyn PcmSource>` without the BFF (or anything downstream) knowing which.
///
/// This sits in `infrastructure` rather than `domain` on purpose: streaming raw
/// PCM bytes is an I/O concern, not domain logic.
pub trait PcmSource: Send + Sync {
    /// Format of the PCM frames, used to build the WAV header. For real capture
    /// this reflects the device's *actual* rate/channels, not what was requested.
    fn format(&self) -> AudioFormat;

    /// Subscribe to the live edge of the capture. Each call yields an
    /// independent receiver; a lagging consumer drops frames rather than
    /// stalling the producer.
    fn subscribe(&self) -> broadcast::Receiver<Bytes>;
}
