//! Scan for a Tesla BLE beacon by VIN.
//!
//! Usage:
//!   cargo run --example scan -- <VIN>
//!
//! Linux: requires bluez running.
//! macOS: requires Bluetooth permission on the terminal app.

use std::time::Duration;

use anyhow::Context;
use sentryusb_tesla_ble::scan;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,btleplug=warn".into()),
        )
        .with_target(false)
        .init();

    let vin = std::env::args()
        .nth(1)
        .context("usage: cargo run --example scan -- <VIN>")?;
    if vin.len() != 17 {
        anyhow::bail!("VIN must be 17 characters, got {}", vin.len());
    }

    let adapter = scan::first_adapter().await?;
    let result = scan::scan_for_vin(&adapter, &vin, Duration::from_secs(30)).await?;

    println!(
        "Scan succeeded: {}  RSSI={}dBm",
        result.local_name,
        result
            .rssi
            .map(|r| r.to_string())
            .unwrap_or_else(|| "?".into())
    );
    Ok(())
}
