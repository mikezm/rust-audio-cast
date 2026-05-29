use std::path::Path;

use serde::Deserialize;

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
    pub default_cast_uuid: Option<String>,
    pub advertised_ip: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            usb_device_name: "USB Audio CODEC".to_string(),
            bind_addr: "0.0.0.0:8080".to_string(),
            sample_rate: 44_100,
            channels: 2,
            discovery_timeout_secs: 4,
            default_cast_uuid: None,
            advertised_ip: None,
        }
    }
}

impl Config {
    /// Resolve config from, in order: `$AUDIOCAST_CONFIG`,
    /// `/etc/audiocast/config.toml`, `./config.toml`, then built-in defaults.
    pub fn load() -> Self {
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
