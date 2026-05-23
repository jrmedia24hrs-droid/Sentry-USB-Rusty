//! Detects whether the car is "awake" by watching the gadget LUN
//! backing file for write activity. Tesla writes to RecentClips every
//! ~60s while the car is on (driving OR parked with dashcam/Sentry).
//! No writes for several minutes = car asleep.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Path to the dashcam disk image — same constant used by the
/// `usb_gadget` crate. The car writes to this file via the USB
/// mass-storage gadget any time it's recording.
pub const CAM_DISK_PATH: &str = "/backingfiles/cam_disk.bin";

/// "Awake" threshold. Tesla writes a new clip ~every minute, so any
/// modification within the last ~90 s is a strong "car is recording"
/// signal. Tuned to tolerate one missed write without bouncing
/// awake → asleep → awake.
const AWAKE_WITHIN: Duration = Duration::from_secs(90);

/// "Asleep" threshold. After 5 minutes of no clip writes, treat the
/// car as fully asleep — drop the sample rate and let the iOS GATT
/// daemon have the radio back. Longer than `AWAKE_WITHIN` to provide
/// hysteresis on flaky write activity.
const ASLEEP_AFTER: Duration = Duration::from_secs(300);

/// Result of one observation of the gadget file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarState {
    /// Recent clip writes — car is on. Sample at the fast cadence.
    Awake,
    /// No writes for `ASLEEP_AFTER` or longer — car is fully asleep.
    /// Sample with `body-controller-state` at the slow cadence.
    Asleep,
    /// Between thresholds (writes within last 5 min but not last 90 s).
    /// Treat as "probably awake" — keep sampling but don't pull the
    /// radio if we don't already hold it. Avoids bouncing the iOS
    /// GATT daemon for transient lulls.
    Idle,
}

/// Inspect the gadget LUN file and decide the car's current state.
/// Missing file is treated as `Asleep` so the daemon doesn't crash
/// in dev / unconfigured Pis.
pub fn observe() -> CarState {
    observe_path(Path::new(CAM_DISK_PATH))
}

pub fn observe_path(path: &Path) -> CarState {
    let mtime = match path.metadata().and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return CarState::Asleep,
    };
    let now = SystemTime::now();
    let age = now.duration_since(mtime).unwrap_or(Duration::ZERO);
    if age < AWAKE_WITHIN {
        CarState::Awake
    } else if age < ASLEEP_AFTER {
        CarState::Idle
    } else {
        CarState::Asleep
    }
}

/// Convenience: unix-seconds mtime, for logging.
#[allow(dead_code)]
pub fn last_write_ts(path: &Path) -> Option<i64> {
    path.metadata()
        .and_then(|m| m.modified())
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn awake_when_freshly_written() {
        let f = tempfile::NamedTempFile::new().unwrap();
        fs::write(f.path(), b"x").unwrap();
        assert_eq!(observe_path(f.path()), CarState::Awake);
    }

    #[test]
    fn asleep_when_missing() {
        let p = std::path::PathBuf::from("/tmp/__telemetry_test_nonexistent_xyz");
        let _ = fs::remove_file(&p);
        assert_eq!(observe_path(&p), CarState::Asleep);
    }
}
