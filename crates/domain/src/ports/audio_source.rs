use crate::model::AudioFormat;

/// The captured USB audio source. The streaming/byte-frame surface is added in
/// Chunk D (cpal adapter); for now it exposes the negotiated capture format.
#[cfg_attr(feature = "mock", mockall::automock)]
pub trait AudioSource: Send + Sync {
    fn format(&self) -> AudioFormat;
}
