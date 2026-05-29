use std::sync::Arc;

use domain::{CastController, CastSession, DomainError};

/// Mute or unmute the device (the `/volume-mute` and `/volume-unmute` endpoints).
pub struct SetMuted {
    cast: Arc<dyn CastController>,
}

impl SetMuted {
    pub fn new(cast: Arc<dyn CastController>) -> Self {
        Self { cast }
    }

    pub async fn execute(
        &self,
        session: &mut CastSession,
        muted: bool,
    ) -> Result<(), DomainError> {
        self.cast.set_muted(muted).await?;
        if muted {
            session.mute();
        } else {
            session.unmute();
        }
        Ok(())
    }
}
