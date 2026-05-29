//! Application layer: one struct per use case, each holding the ports it needs
//! and exposing a single `execute`/`query`. No knowledge of HTTP, cpal, or the
//! cast protocol — it orchestrates the domain through ports the composition root
//! wires up. This is the seam the BFF (axum) layer calls into.

pub mod adjust_volume;
pub mod refresh_devices;
pub mod set_muted;
pub mod set_volume;
pub mod start_casting;
pub mod status;
pub mod stop_casting;

pub use adjust_volume::{AdjustVolume, Direction};
pub use refresh_devices::RefreshDevices;
pub use set_muted::SetMuted;
pub use set_volume::SetVolume;
pub use start_casting::StartCasting;
pub use status::{GetStatus, StatusReport};
pub use stop_casting::StopCasting;
