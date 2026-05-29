use std::sync::Mutex;

use async_trait::async_trait;
use domain::{AudioFormat, CastController, DeviceId, DiscoveredDevice, DomainError, StreamUrl, Volume};

/// A fake `CastController` that records what it was told to do and always
/// succeeds. Lets the full control plane run on a dev machine with no real
/// Chromecast. Replaced by the rust_cast adapter in Chunk E.
#[derive(Default)]
pub struct InMemoryCastController {
    connected: Mutex<Option<DeviceId>>,
}

impl InMemoryCastController {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl CastController for InMemoryCastController {
    async fn connect(&self, device: &DiscoveredDevice) -> Result<(), DomainError> {
        tracing::info!(device = %device.id, name = %device.name, "fake cast: connect");
        *self.connected.lock().unwrap() = Some(device.id.clone());
        Ok(())
    }

    async fn play(&self, url: &StreamUrl, format: AudioFormat) -> Result<(), DomainError> {
        tracing::info!(%url, ?format, "fake cast: play");
        Ok(())
    }

    async fn stop(&self) -> Result<(), DomainError> {
        tracing::info!("fake cast: stop");
        Ok(())
    }

    async fn set_volume(&self, volume: Volume) -> Result<(), DomainError> {
        tracing::info!(volume = volume.as_f32(), "fake cast: set_volume");
        Ok(())
    }

    async fn set_muted(&self, muted: bool) -> Result<(), DomainError> {
        tracing::info!(muted, "fake cast: set_muted");
        Ok(())
    }
}
