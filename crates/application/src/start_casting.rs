use std::sync::Arc;

use domain::{
    AudioFormat, CastController, CastSession, DeviceId, DiscoveredDevice, DomainError, StreamUrl,
};

/// Resolve a device by id from the discovery catalog, connect, and start playing
/// the stream. An unknown id is rejected before any cast call is made.
pub struct StartCasting {
    cast: Arc<dyn CastController>,
}

impl StartCasting {
    pub fn new(cast: Arc<dyn CastController>) -> Self {
        Self { cast }
    }

    pub async fn execute(
        &self,
        session: &mut CastSession,
        catalog: &[DiscoveredDevice],
        device_id: &DeviceId,
        stream_url: &StreamUrl,
        format: AudioFormat,
    ) -> Result<(), DomainError> {
        let device = catalog
            .iter()
            .find(|d| &d.id == device_id)
            .cloned()
            .ok_or_else(|| DomainError::DeviceNotFound(device_id.clone()))?;

        self.cast.connect(&device).await?;
        self.cast.play(stream_url, format).await?;

        session.select(device);
        session.start_playing()?;
        Ok(())
    }
}
