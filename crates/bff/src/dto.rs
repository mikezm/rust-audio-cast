use application::StatusReport;
use domain::{CastState, DiscoveredDevice};
use serde::Serialize;

/// A discovered speaker, shaped for the UI's speaker picker.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDto {
    pub uuid: String,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub model: Option<String>,
}

impl From<&DiscoveredDevice> for DeviceDto {
    fn from(d: &DiscoveredDevice) -> Self {
        Self {
            uuid: d.id.as_str().to_string(),
            name: d.name.clone(),
            ip: d.ip.clone(),
            port: d.port,
            model: d.model.clone(),
        }
    }
}

/// Current playback state for the `/status` endpoint and after mutations.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusDto {
    pub state: &'static str,
    pub device: Option<DeviceDto>,
    pub volume: f32,
    pub muted: bool,
    pub stream_ready: bool,
}

impl From<StatusReport> for StatusDto {
    fn from(r: StatusReport) -> Self {
        Self {
            state: state_str(r.state),
            device: r.device.as_ref().map(DeviceDto::from),
            volume: r.volume.as_f32(),
            muted: r.muted,
            stream_ready: true,
        }
    }
}

/// Minimal response for the volume endpoints.
#[derive(Debug, Serialize)]
pub struct VolumeDto {
    pub volume: f32,
}

fn state_str(state: CastState) -> &'static str {
    match state {
        CastState::Inactive => "inactive",
        CastState::Connected => "connected",
        CastState::Playing => "playing",
    }
}
