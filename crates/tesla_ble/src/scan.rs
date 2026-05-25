//! Scan for a specific Tesla car's BLE beacon via btleplug.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use tokio::time::{sleep, timeout};
use tracing::{debug, info, warn};

use crate::local_name::{looks_like_tesla_name, vehicle_local_name};

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub peripheral: Peripheral,
    pub local_name: String,
    /// dBm. Higher (less negative) = stronger.
    pub rssi: Option<i16>,
}

/// First BLE adapter btleplug finds. On Linux this is typically `hci0`.
pub async fn first_adapter() -> Result<Adapter> {
    let manager = Manager::new()
        .await
        .context("creating btleplug Manager")?;
    let adapters = manager
        .adapters()
        .await
        .context("listing BLE adapters")?;
    adapters
        .into_iter()
        .next()
        .context("no BLE adapter available — is bluez running?")
}

/// Scan until the given VIN's beacon is found or `timeout_dur` elapses.
pub async fn scan_for_vin(
    adapter: &Adapter,
    vin: &str,
    timeout_dur: Duration,
) -> Result<ScanResult> {
    let expected_name = vehicle_local_name(vin);
    info!("scanning for Tesla (expecting beacon name {})", expected_name);

    let mut events = adapter
        .events()
        .await
        .context("subscribing to btleplug events")?;
    adapter
        .start_scan(ScanFilter::default())
        .await
        .context("starting BLE scan")?;

    let scan_result = timeout(timeout_dur, async {
        while let Some(event) = events.next().await {
            let id = match event {
                CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => id,
                _ => continue,
            };
            let Ok(peripheral) = adapter.peripheral(&id).await else {
                continue;
            };
            let Ok(Some(props)) = peripheral.properties().await else {
                continue;
            };
            let Some(name) = props.local_name else { continue };

            if name == expected_name {
                info!("found target vehicle (RSSI {:?})", props.rssi);
                return Ok(ScanResult {
                    peripheral,
                    local_name: name,
                    rssi: props.rssi,
                });
            } else if looks_like_tesla_name(&name) {
                debug!("ignoring different Tesla beacon (RSSI {:?})", props.rssi);
            }
        }
        bail!("event stream ended before finding vehicle")
    })
    .await;

    if let Err(e) = adapter.stop_scan().await {
        warn!("stop_scan failed (continuing): {e}");
    }

    match scan_result {
        Ok(Ok(r)) => Ok(r),
        Ok(Err(e)) => Err(e),
        Err(_elapsed) => bail!(
            "scan timed out after {:?} — car may be out of range, asleep, or advertising-paused",
            timeout_dur
        ),
    }
}

/// One-shot scan using the first available adapter, 30s timeout.
pub async fn scan_default(vin: &str) -> Result<ScanResult> {
    let adapter = first_adapter().await?;
    sleep(Duration::from_millis(200)).await;
    scan_for_vin(&adapter, vin, Duration::from_secs(30)).await
}
