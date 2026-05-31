use std::sync::Arc;

use bff::AudioStreamProvider;
use bytes::Bytes;
use domain::AudioFormat;
use futures::stream::BoxStream;
use futures::StreamExt;
use infrastructure::PcmSource;
use tokio_stream::wrappers::BroadcastStream;

/// Composition-root glue adapting an infrastructure audio source to the BFF's
/// `AudioStreamProvider` port. A local newtype keeps both adapter crates
/// independent of each other (the trait lives in `bff`, the source in
/// `infrastructure`) while satisfying the orphan rule. The source is a
/// `dyn PcmSource`, so the synthetic tone and the real cpal capture are
/// interchangeable here.
pub struct AudioHandle(pub Arc<dyn PcmSource>);

impl AudioStreamProvider for AudioHandle {
    fn format(&self) -> AudioFormat {
        self.0.format()
    }

    fn subscribe(&self) -> BoxStream<'static, Bytes> {
        // Drop lag errors: a slow client just loses frames and resumes at the
        // live edge, rather than stalling capture.
        BroadcastStream::new(self.0.subscribe())
            .filter_map(|r| async move { r.ok() })
            .boxed()
    }
}
