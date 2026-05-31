//! Composition root: load config, construct adapters, wire them into the BFF,
//! and serve. This is the only place that knows about concrete adapters.

mod audio_adapter;
mod config;

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use audio_adapter::AudioHandle;
use bff::{AppState, AudioStreamProvider};
use config::{AudioBackend, Backend, Config};
use domain::{AudioFormat, CastController, CastSession, DeviceDiscovery, StreamUrl};
use infrastructure::{
    CpalAudioStream, InMemoryCastController, MdnsDiscovery, PcmSource, RustCastController,
    StaticDiscovery, SyntheticAudioStream,
};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=warn".into()),
        )
        .init();

    let config = Config::load();
    let requested = AudioFormat {
        sample_rate: config.sample_rate,
        channels: config.channels,
        bits_per_sample: 16,
    };

    // Audio source: synthetic tone, or real USB capture (per config / env).
    let audio: Arc<dyn PcmSource> = match config.audio {
        AudioBackend::Synthetic => SyntheticAudioStream::start(requested),
        AudioBackend::Capture => CpalAudioStream::start(config.usb_device_name.clone())?,
    };
    tracing::info!(audio = ?config.audio, "audio backend");

    // The real device may not honor the requested rate/channels, so the header
    // and stream URL follow the source's *actual* format.
    let format = audio.format();
    let audio_provider: Arc<dyn AudioStreamProvider> = Arc::new(AudioHandle(audio));

    // Control + discovery: fakes, or real rust_cast/mdns, per config.
    let (cast, discovery): (Arc<dyn CastController>, Arc<dyn DeviceDiscovery>) =
        match config.backend {
            Backend::Fake => (
                Arc::new(InMemoryCastController::new()),
                Arc::new(StaticDiscovery::new()),
            ),
            Backend::Live => (
                Arc::new(RustCastController::new()),
                Arc::new(MdnsDiscovery::new()),
            ),
        };
    tracing::info!(backend = ?config.backend, "adapter backend");

    let stream_url = build_stream_url(&config);
    tracing::info!(%stream_url, "audio stream url");

    let state = AppState {
        cast,
        discovery,
        audio: audio_provider,
        session: Arc::new(Mutex::new(CastSession::new())),
        catalog: Arc::new(Mutex::new(Vec::new())),
        stream_url,
        format,
        discovery_timeout: Duration::from_secs(config.discovery_timeout_secs),
        web_root: config.web_root.clone().into(),
    };

    let app = bff::router(state);
    let listener = TcpListener::bind(&config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Build the URL the Chromecast will fetch the stream from, using the configured
/// or auto-detected local IP and the bound port (omitted for port 80).
fn build_stream_url(config: &Config) -> StreamUrl {
    let ip = config
        .advertised_ip
        .clone()
        .unwrap_or_else(|| detect_ip().to_string());
    let port = config
        .bind_addr
        .rsplit(':')
        .next()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    let host = if port == 80 { ip } else { format!("{ip}:{port}") };
    StreamUrl::new(format!("http://{host}/audio-stream"))
}

fn detect_ip() -> IpAddr {
    local_ip_address::local_ip().unwrap_or_else(|_| IpAddr::from([127, 0, 0, 1]))
}
