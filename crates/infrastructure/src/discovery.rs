use std::time::Duration;

use async_trait::async_trait;
use domain::{DeviceDiscovery, DeviceId, DiscoveredDevice, DomainError};

/// A fake discovery adapter returning a fixed set of devices, so `/list-casts`
/// and `/start-cast` work without real Chromecasts on the network. Replaced by
/// the mdns-sd adapter in Chunk E.
#[derive(Default)]
pub struct StaticDiscovery;

impl StaticDiscovery {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DeviceDiscovery for StaticDiscovery {
    async fn discover(&self, _timeout: Duration) -> Result<Vec<DiscoveredDevice>, DomainError> {
        Ok(vec![
            DiscoveredDevice {
                id: DeviceId::new("fake-living-room"),
                name: "Living Room (fake)".to_string(),
                ip: "192.168.1.21".to_string(),
                port: 8009,
                model: Some("Chromecast Audio".to_string()),
            },
            DiscoveredDevice {
                id: DeviceId::new("fake-kitchen"),
                name: "Kitchen (fake)".to_string(),
                ip: "192.168.1.22".to_string(),
                port: 8009,
                model: Some("Google Home".to_string()),
            },
        ])
    }
}
