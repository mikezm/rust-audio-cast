use crate::model::DeviceId;

/// Errors surfaced by the domain and its ports. Adapters map their own
/// library errors (`cpal::Error`, `rust_cast::Error`, …) into these variants
/// at the boundary, so nothing outside `infrastructure` ever sees an infra type.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("device not found: {0}")]
    DeviceNotFound(DeviceId),

    #[error("no device selected")]
    NoDeviceSelected,

    #[error("cast device error: {0}")]
    Cast(String),

    #[error("discovery error: {0}")]
    Discovery(String),

    #[error("audio error: {0}")]
    Audio(String),
}
