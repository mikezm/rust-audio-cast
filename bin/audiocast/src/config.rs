use std::path::Path;

use serde::Deserialize;

/// Which control/discovery backend to wire up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    /// In-memory cast + static device list. No hardware needed.
    Fake,
    /// Real rust_cast control + mdns-sd discovery.
    Live,
}

/// Which audio source to capture from. Chosen independently of the control
/// backend, so you can cast a real turntable while testing, or stream the
/// synthetic tone with no USB device attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioBackend {
    /// 440 Hz sine tone. No hardware needed.
    Synthetic,
    /// Real USB capture via cpal/ALSA.
    Capture,
}

/// Runtime configuration. Every field has a default, so a missing config file
/// is fine for local dev.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub usb_device_name: String,
    pub bind_addr: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub discovery_timeout_secs: u64,
    pub backend: Backend,
    pub audio: AudioBackend,
    pub web_root: String,
    pub default_cast_uuid: Option<String>,
    pub advertised_ip: Option<String>,
    /// Stop an active cast after this many seconds of continuous input
    /// silence (the record ran out). 0 disables auto-stop.
    pub silence_stop_secs: u64,
    /// RMS level (fraction of full scale, 0.0–1.0) below which a second of
    /// audio counts as silent.
    pub silence_threshold: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            usb_device_name: "USB Audio CODEC".to_string(),
            bind_addr: "0.0.0.0:8080".to_string(),
            sample_rate: 44_100,
            channels: 2,
            discovery_timeout_secs: 4,
            backend: Backend::Fake,
            audio: AudioBackend::Synthetic,
            web_root: "www".to_string(),
            default_cast_uuid: None,
            advertised_ip: None,
            silence_stop_secs: 120,
            silence_threshold: 0.005,
        }
    }
}

impl Config {
    /// Resolve config from, in order: `$AUDIOCAST_CONFIG`,
    /// `/etc/audiocast/config.toml`, `./config.toml`, then built-in defaults.
    /// `AUDIOCAST_BACKEND=live|fake` and `AUDIOCAST_AUDIO=capture|synthetic`
    /// override the respective backends for quick testing.
    pub fn load() -> Self {
        let mut config = Self::from_sources();
        if let Ok(b) = std::env::var("AUDIOCAST_BACKEND") {
            config.backend = match b.to_lowercase().as_str() {
                "live" => Backend::Live,
                _ => Backend::Fake,
            };
        }
        if let Ok(a) = std::env::var("AUDIOCAST_AUDIO") {
            config.audio = match a.to_lowercase().as_str() {
                "capture" => AudioBackend::Capture,
                _ => AudioBackend::Synthetic,
            };
        }
        config
    }

    fn from_sources() -> Self {
        if let Ok(path) = std::env::var("AUDIOCAST_CONFIG") {
            return Self::from_path(&path);
        }
        for candidate in ["/etc/audiocast/config.toml", "config.toml"] {
            if Path::new(candidate).exists() {
                return Self::from_path(candidate);
            }
        }
        Config::default()
    }

    fn from_path(path: &str) -> Self {
        match std::fs::read_to_string(path).map(|s| toml::from_str::<Config>(&s)) {
            Ok(Ok(config)) => {
                tracing::info!(path, "loaded config");
                config
            }
            Ok(Err(e)) => {
                tracing::warn!(path, error = %e, "invalid config; using defaults");
                Config::default()
            }
            Err(e) => {
                tracing::warn!(path, error = %e, "could not read config; using defaults");
                Config::default()
            }
        }
    }
}
