use std::sync::Arc;

use domain::{CastController, CastSession, DomainError, Volume};

/// Direction for a relative volume change (the `/volume-up` and `/volume-down`
/// endpoints).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

/// Step the volume one increment up or down (clamped by `Volume`).
pub struct AdjustVolume {
    cast: Arc<dyn CastController>,
}

impl AdjustVolume {
    pub fn new(cast: Arc<dyn CastController>) -> Self {
        Self { cast }
    }

    pub async fn execute(
        &self,
        session: &mut CastSession,
        direction: Direction,
    ) -> Result<Volume, DomainError> {
        let next = match direction {
            Direction::Up => session.volume().up(),
            Direction::Down => session.volume().down(),
        };
        self.cast.set_volume(next).await?;
        session.set_volume(next);
        Ok(next)
    }
}
