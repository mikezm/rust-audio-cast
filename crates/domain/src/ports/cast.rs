use async_trait::async_trait;

use crate::error::DomainError;
use crate::model::{AudioFormat, DiscoveredDevice, StreamUrl, Volume};

/// Drives a Chromecast: connect, play a stream URL, control volume/mute, stop.
/// Implemented by the rust_cast adapter; mocked in application tests.
#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait CastController: Send + Sync {
    async fn connect(&self, device: &DiscoveredDevice) -> Result<(), DomainError>;
    async fn play(&self, url: &StreamUrl, format: AudioFormat) -> Result<(), DomainError>;
    async fn stop(&self) -> Result<(), DomainError>;
    async fn set_volume(&self, volume: Volume) -> Result<(), DomainError>;
    async fn set_muted(&self, muted: bool) -> Result<(), DomainError>;
}
