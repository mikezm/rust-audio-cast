use std::sync::Arc;

use domain::{CastController, CastSession, DomainError};

/// Stop playback on the current device.
pub struct StopCasting {
    cast: Arc<dyn CastController>,
}

impl StopCasting {
    pub fn new(cast: Arc<dyn CastController>) -> Self {
        Self { cast }
    }

    pub async fn execute(&self, session: &mut CastSession) -> Result<(), DomainError> {
        self.cast.stop().await?;
        session.stop_playing();
        Ok(())
    }
}
