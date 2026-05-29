use std::sync::Arc;

use domain::{CastController, CastSession, DomainError, Volume};

/// Set an absolute volume on the device and record it on the session.
pub struct SetVolume {
    cast: Arc<dyn CastController>,
}

impl SetVolume {
    pub fn new(cast: Arc<dyn CastController>) -> Self {
        Self { cast }
    }

    pub async fn execute(
        &self,
        session: &mut CastSession,
        volume: Volume,
    ) -> Result<Volume, DomainError> {
        self.cast.set_volume(volume).await?;
        session.set_volume(volume);
        Ok(session.volume())
    }
}
