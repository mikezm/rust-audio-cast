use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use domain::{
    AudioFormat, CastController, CastSession, DeviceDiscovery, DiscoveredDevice, StreamUrl,
};
use tokio::sync::Mutex;

use crate::audio::AudioStreamProvider;

/// Shared application state handed to every handler. Holds the ports (wired by
/// the composition root) plus the mutable session and discovery cache behind
/// async mutexes. Cheap to clone — every field is an `Arc` or `Copy`.
#[derive(Clone)]
pub struct AppState {
    pub cast: Arc<dyn CastController>,
    pub discovery: Arc<dyn DeviceDiscovery>,
    pub audio: Arc<dyn AudioStreamProvider>,
    pub session: Arc<Mutex<CastSession>>,
    pub catalog: Arc<Mutex<Vec<DiscoveredDevice>>>,
    pub stream_url: StreamUrl,
    pub format: AudioFormat,
    pub discovery_timeout: Duration,
    /// Directory the static touch UI is served from. Relative paths resolve
    /// against the process cwd, so under a systemd service this should be an
    /// absolute path (e.g. `/usr/share/audiocast/www`).
    pub web_root: PathBuf,
}
