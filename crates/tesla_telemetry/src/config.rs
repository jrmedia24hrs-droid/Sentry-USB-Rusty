//! Reads the bits of `sentryusb.conf` the telemetry sampler cares
//! about: the master BLE toggle and the Tesla VIN. Re-evaluated on
//! every main-loop iteration so the daemon picks up settings changes
//! without a restart.

use anyhow::Result;

/// Snapshot of the BLE-relevant config values.
#[derive(Debug, Clone, Default)]
pub struct BleConfig {
    pub enabled: bool,
    pub vin: String,
}

impl BleConfig {
    /// Read the current config. Defaults to a permissive "enabled+VIN
    /// set" interpretation matching the api crate's `is_ble_enabled`
    /// resolution order so behavior is consistent across surfaces.
    pub fn load() -> Result<Self> {
        let config_path = sentryusb_config::find_config_path();
        let (active, commented) = sentryusb_config::parse_file(config_path)?;

        // BLE_ENABLED resolution mirrors api/src/ble.rs::is_ble_enabled.
        let enabled =
            match sentryusb_config::get_config_value(&active, &commented, "BLE_ENABLED") {
                Some(v) => matches!(v.as_str(), "yes" | "true" | "1"),
                None => {
                    // Unset → legacy default: implicit "yes" if user
                    // previously configured BLE (VIN present or paired
                    // marker), else "no".
                    active.contains_key("TESLA_BLE_VIN")
                        || std::path::Path::new("/root/.ble/paired").exists()
                }
            };

        let vin = active
            .get("TESLA_BLE_VIN")
            .cloned()
            .unwrap_or_default()
            .to_uppercase();

        Ok(Self { enabled, vin })
    }
}
