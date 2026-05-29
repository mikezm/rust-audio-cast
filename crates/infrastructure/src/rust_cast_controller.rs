use std::sync::Mutex;

use async_trait::async_trait;
use domain::{AudioFormat, CastController, DiscoveredDevice, DomainError, StreamUrl, Volume};
use rust_cast::channels::media::{Media, StreamType};
use rust_cast::channels::receiver::CastDeviceApp;
use rust_cast::CastDevice;

/// The Chromecast platform receiver destination.
const PLATFORM_RECEIVER: &str = "receiver-0";

/// Real Chromecast control via `rust_cast`.
///
/// Each operation opens a short-lived connection, issues its (synchronous)
/// command, and drops it. Casting persists on the device after the sender
/// disconnects, so we avoid holding a long-lived connection and managing
/// heartbeats. `rust_cast` is blocking and `CastDevice` is `!Send`, so every op
/// runs inside `spawn_blocking` and the device never crosses an await.
#[derive(Default)]
pub struct RustCastController {
    endpoint: Mutex<Option<(String, u16)>>,
}

impl RustCastController {
    pub fn new() -> Self {
        Self::default()
    }

    fn endpoint(&self) -> Result<(String, u16), DomainError> {
        self.endpoint
            .lock()
            .unwrap()
            .clone()
            .ok_or(DomainError::NoDeviceSelected)
    }
}

#[async_trait]
impl CastController for RustCastController {
    async fn connect(&self, device: &DiscoveredDevice) -> Result<(), DomainError> {
        *self.endpoint.lock().unwrap() = Some((device.ip.clone(), device.port));
        tracing::info!(device = %device.id, ip = %device.ip, port = device.port, "cast: selected");
        Ok(())
    }

    async fn play(&self, url: &StreamUrl, _format: AudioFormat) -> Result<(), DomainError> {
        let (host, port) = self.endpoint()?;
        let url = url.as_str().to_string();
        run_blocking(move || cast_play(&host, port, &url)).await
    }

    async fn stop(&self) -> Result<(), DomainError> {
        let (host, port) = self.endpoint()?;
        run_blocking(move || cast_stop(&host, port)).await
    }

    async fn set_volume(&self, volume: Volume) -> Result<(), DomainError> {
        let (host, port) = self.endpoint()?;
        let level = volume.as_f32();
        run_blocking(move || cast_set_volume(&host, port, level)).await
    }

    async fn set_muted(&self, muted: bool) -> Result<(), DomainError> {
        let (host, port) = self.endpoint()?;
        run_blocking(move || cast_set_muted(&host, port, muted)).await
    }
}

/// Run a blocking cast op off the async runtime. The closure creates and drops
/// the `!Send` `CastDevice` entirely within the blocking thread.
async fn run_blocking<F>(f: F) -> Result<(), DomainError>
where
    F: FnOnce() -> Result<(), DomainError> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| DomainError::Cast(format!("cast task failed: {e}")))?
}

fn err(e: rust_cast::errors::Error) -> DomainError {
    DomainError::Cast(format!("{e:?}"))
}

/// Connect to the device (Chromecasts use self-signed certs, so verification is
/// skipped) and attach to the platform receiver. An owned host string keeps the
/// returned device `'static`.
fn connect_device(host: &str, port: u16) -> Result<CastDevice<'static>, DomainError> {
    let device = CastDevice::connect_without_host_verification(host.to_string(), port)
        .map_err(err)?;
    device.connection.connect(PLATFORM_RECEIVER).map_err(err)?;
    Ok(device)
}

fn cast_play(host: &str, port: u16, url: &str) -> Result<(), DomainError> {
    let device = connect_device(host, port)?;
    let app = device
        .receiver
        .launch_app(&CastDeviceApp::DefaultMediaReceiver)
        .map_err(err)?;
    device
        .connection
        .connect(app.transport_id.clone())
        .map_err(err)?;

    let media = Media {
        content_id: url.to_string(),
        stream_type: StreamType::Live,
        content_type: AudioFormat::MIME.to_string(),
        metadata: None,
        duration: None,
    };
    device
        .media
        .load(app.transport_id.clone(), app.session_id.clone(), &media)
        .map_err(err)?;
    tracing::info!(%url, "cast: playing");
    Ok(())
}

fn cast_set_volume(host: &str, port: u16, level: f32) -> Result<(), DomainError> {
    let device = connect_device(host, port)?;
    device.receiver.set_volume(level).map_err(err)?;
    Ok(())
}

fn cast_set_muted(host: &str, port: u16, muted: bool) -> Result<(), DomainError> {
    let device = connect_device(host, port)?;
    device.receiver.set_volume(muted).map_err(err)?;
    Ok(())
}

fn cast_stop(host: &str, port: u16) -> Result<(), DomainError> {
    let device = connect_device(host, port)?;
    let status = device.receiver.get_status().map_err(err)?;
    if let Some(app) = status.applications.into_iter().next() {
        device.receiver.stop_app(app.session_id).map_err(err)?;
    }
    tracing::info!("cast: stopped");
    Ok(())
}
