use std::collections::HashMap;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use domain::{DeviceDiscovery, DeviceId, DiscoveredDevice, DomainError};
use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent};

const SERVICE: &str = "_googlecast._tcp.local.";

/// Real LAN discovery of Chromecast devices via mDNS (`mdns-sd`). The browse
/// loop is blocking, so it runs on `spawn_blocking`.
#[derive(Default)]
pub struct MdnsDiscovery;

impl MdnsDiscovery {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DeviceDiscovery for MdnsDiscovery {
    async fn discover(&self, timeout: Duration) -> Result<Vec<DiscoveredDevice>, DomainError> {
        tokio::task::spawn_blocking(move || browse(timeout))
            .await
            .map_err(|e| DomainError::Discovery(format!("discovery task failed: {e}")))?
    }
}

fn browse(timeout: Duration) -> Result<Vec<DiscoveredDevice>, DomainError> {
    let daemon = ServiceDaemon::new().map_err(|e| DomainError::Discovery(format!("{e}")))?;
    let receiver = daemon
        .browse(SERVICE)
        .map_err(|e| DomainError::Discovery(format!("{e}")))?;

    let deadline = Instant::now() + timeout;
    // Keyed by Chromecast id so repeated resolves of the same device collapse.
    let mut found: HashMap<String, DiscoveredDevice> = HashMap::new();

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                if let Some(device) = to_device(&info) {
                    found.insert(device.id.as_str().to_string(), device);
                }
            }
            Ok(_) => {}
            // Timeout or the browse channel closed: we're done collecting.
            Err(_) => break,
        }
    }

    let _ = daemon.shutdown();
    Ok(found.into_values().collect())
}

/// Map a resolved mDNS service to our domain device. The Chromecast TXT record
/// carries `id` (UUID), `fn` (friendly name), and `md` (model).
fn to_device(info: &ResolvedService) -> Option<DiscoveredDevice> {
    let id = info.get_property_val_str("id")?;
    let ip = info.get_addresses_v4().into_iter().next()?;
    let name = info
        .get_property_val_str("fn")
        .map(str::to_string)
        .unwrap_or_else(|| info.get_fullname().to_string());
    let model = info.get_property_val_str("md").map(str::to_string);

    Some(DiscoveredDevice {
        id: DeviceId::new(id),
        name,
        ip: ip.to_string(),
        port: info.get_port(),
        model,
    })
}
