use std::sync::Arc;
use std::time::Duration;

use domain::{DeviceDiscovery, DiscoveredDevice, DomainError};

/// Re-run mDNS discovery and return the devices found (the `/refresh-casts`
/// endpoint). The caller caches the result for `/list-casts`.
pub struct RefreshDevices {
    discovery: Arc<dyn DeviceDiscovery>,
}

impl RefreshDevices {
    pub fn new(discovery: Arc<dyn DeviceDiscovery>) -> Self {
        Self { discovery }
    }

    pub async fn execute(&self, timeout: Duration) -> Result<Vec<DiscoveredDevice>, DomainError> {
        self.discovery.discover(timeout).await
    }
}
