use super::{DiscoveredDevice, Volume};
use crate::error::DomainError;

/// Playback state of the casting session, mirroring the original app's
/// INACTIVE / NEW / ACTIVE statuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastState {
    /// No active playback (nothing selected, or stopped).
    Inactive,
    /// A device is selected/connected but not currently playing.
    Connected,
    /// Streaming to the selected device.
    Playing,
}

/// The aggregate root: what we are casting to, the target volume, and whether
/// we're muted. Pure state + transition rules — directly unit-testable, with no
/// dependency on the cast protocol or audio I/O.
#[derive(Debug, Clone)]
pub struct CastSession {
    device: Option<DiscoveredDevice>,
    state: CastState,
    volume: Volume,
    muted: bool,
}

impl CastSession {
    pub fn new() -> Self {
        Self {
            device: None,
            state: CastState::Inactive,
            volume: Volume::default(),
            muted: false,
        }
    }

    /// Select (connect to) a device without yet playing.
    pub fn select(&mut self, device: DiscoveredDevice) {
        self.device = Some(device);
        self.state = CastState::Connected;
    }

    /// Transition to playing. Requires a selected device.
    pub fn start_playing(&mut self) -> Result<(), DomainError> {
        if self.device.is_none() {
            return Err(DomainError::NoDeviceSelected);
        }
        self.state = CastState::Playing;
        Ok(())
    }

    /// Stop playback. The selected device is retained for status/replay.
    pub fn stop_playing(&mut self) {
        self.state = CastState::Inactive;
    }

    /// Set the target volume. Setting a volume explicitly clears mute, since the
    /// user dragging the slider implies they want to hear it.
    pub fn set_volume(&mut self, volume: Volume) {
        self.volume = volume;
        self.muted = false;
    }

    pub fn mute(&mut self) {
        self.muted = true;
    }

    pub fn unmute(&mut self) {
        self.muted = false;
    }

    pub fn device(&self) -> Option<&DiscoveredDevice> {
        self.device.as_ref()
    }

    pub fn state(&self) -> CastState {
        self.state
    }

    /// The target volume, independent of mute. Unmuting restores exactly this.
    pub fn volume(&self) -> Volume {
        self.volume
    }

    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// What the speaker should actually output right now: zero while muted,
    /// otherwise the target volume.
    pub fn effective_volume(&self) -> Volume {
        if self.muted {
            Volume::new(0.0)
        } else {
            self.volume
        }
    }
}

impl Default for CastSession {
    fn default() -> Self {
        Self::new()
    }
}
