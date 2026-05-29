use bytes::Bytes;
use domain::AudioFormat;
use futures::stream::BoxStream;

/// The audio data plane, kept out of the domain on purpose: streaming raw PCM
/// bytes is an infrastructure↔HTTP concern, not domain logic. The BFF consumes
/// this port directly; the composition root adapts a concrete source (synthetic
/// tone now, cpal later) to it.
pub trait AudioStreamProvider: Send + Sync {
    /// Format of the PCM frames, used to build the WAV header.
    fn format(&self) -> AudioFormat;

    /// Subscribe to the live edge of the capture. Each call yields an
    /// independent stream of PCM byte frames; lagging consumers drop frames.
    fn subscribe(&self) -> BoxStream<'static, Bytes>;
}
