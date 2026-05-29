use domain::{CastSession, CastState, DiscoveredDevice, Volume};

/// A snapshot of the session for the `/status` endpoint. The BFF maps this into
/// its UI-shaped DTO.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusReport {
    pub state: CastState,
    pub device: Option<DiscoveredDevice>,
    pub volume: Volume,
    pub muted: bool,
}

/// Read-only query over the current session.
pub struct GetStatus;

impl GetStatus {
    pub fn execute(session: &CastSession) -> StatusReport {
        StatusReport {
            state: session.state(),
            device: session.device().cloned(),
            volume: session.volume(),
            muted: session.is_muted(),
        }
    }
}
