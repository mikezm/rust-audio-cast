use std::time::Duration;

use async_trait::async_trait;

use crate::error::DomainError;
use crate::model::DiscoveredDevice;

/// Discovers Chromecast devices on the LAN. Implemented by the mdns-sd adapter.
#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait DeviceDiscovery: Send + Sync {
    async fn discover(&self, timeout: Duration) -> Result<Vec<DiscoveredDevice>, DomainError>;
}
