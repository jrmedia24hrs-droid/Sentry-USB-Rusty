//! BLE radio lock — a single file under `/tmp` that says who owns
//! `hci0` right now.
//!
//! Three potential users compete for the radio: this telemetry
//! sampler, the keep-awake nudge loop ([`run/awake_start`]), and the
//! iOS-app GATT daemon (`sentryusb-ble.service`). On a Pi's single
//! controller, two centrals can't reliably run simultaneously, and a
//! central + peripheral coexistence pattern works but is fiddly with
//! BlueZ. So we serialize: whoever holds the lock has exclusive
//! access; the daemon stops `sentryusb-ble` while held, restarts it
//! on release.
//!
//! The file lives at `/tmp/ble_radio_owner` and contains
//! `<owner-name>\n<unix_seconds>\n`. Both the bash keep-awake script
//! and this daemon read/write it. A stale lock (>24h old) is treated
//! as crashed and re-claimed.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use tracing::{info, warn};

/// Shared lock path. Coordinated with [`run/awake_start`]'s
/// `BLE_LOCK` constant — keep the two in sync.
pub const LOCK_PATH: &str = "/tmp/ble_radio_owner";

/// Stale-lock threshold. Matches `BLE_LOCK_MAX_AGE` in `awake_start`.
const STALE_AFTER_SECS: i64 = 86_400;

/// Acquire the radio lock for `owner`. Returns `true` if we now hold
/// it. Returns `false` if another fresh owner holds it — callers
/// should back off and retry later.
///
/// Best-effort, not strictly atomic: there's a small race between the
/// "is it stale?" check and the write. In practice the three callers
/// (keep-awake, telemetry, future) don't fight in tight loops — they
/// hold the lock for seconds-to-minutes at a time, and the cost of a
/// lost race is just one extra retry on the next 5-second tick.
pub fn try_acquire(owner: &str) -> Result<bool> {
    let now = now_secs();

    if Path::new(LOCK_PATH).exists() {
        match read_lock() {
            Ok((existing_owner, ts)) => {
                if existing_owner == owner {
                    // Re-acquire — refresh the timestamp so a long
                    // hold doesn't appear stale.
                    write_lock(owner, now)?;
                    return Ok(true);
                }
                if now - ts > STALE_AFTER_SECS {
                    warn!(
                        "BLE radio lock held by '{}' for {}s — assuming crashed, taking over",
                        existing_owner,
                        now - ts
                    );
                    write_lock(owner, now)?;
                    return Ok(true);
                }
                return Ok(false);
            }
            Err(e) => {
                warn!("BLE radio lock file unreadable ({}) — overwriting", e);
                write_lock(owner, now)?;
                return Ok(true);
            }
        }
    }

    write_lock(owner, now)?;
    info!("BLE radio lock acquired by '{}'", owner);
    Ok(true)
}

/// Release the radio lock if we own it. No-op if the file is missing
/// or owned by someone else (some other component may have taken it
/// over due to staleness).
pub fn release(owner: &str) -> Result<()> {
    if !Path::new(LOCK_PATH).exists() {
        return Ok(());
    }
    match read_lock() {
        Ok((existing_owner, _)) if existing_owner == owner => {
            fs::remove_file(LOCK_PATH)
                .with_context(|| format!("failed to remove {}", LOCK_PATH))?;
            info!("BLE radio lock released by '{}'", owner);
        }
        Ok((other, _)) => {
            warn!(
                "BLE radio lock owned by '{}', not us ('{}') — not releasing",
                other, owner
            );
        }
        Err(e) => {
            warn!("BLE radio lock unreadable on release ({}) — leaving alone", e);
        }
    }
    Ok(())
}

/// Returns the current owner string if the lock exists, else `None`.
/// Diagnostic helper — never use this to decide whether to acquire
/// (use [`try_acquire`] for that, which handles staleness).
pub fn current_owner() -> Option<String> {
    read_lock().ok().map(|(owner, _)| owner)
}

fn read_lock() -> Result<(String, i64)> {
    let contents = fs::read_to_string(LOCK_PATH)
        .with_context(|| format!("failed to read {}", LOCK_PATH))?;
    let mut lines = contents.lines();
    let owner = lines
        .next()
        .ok_or_else(|| anyhow!("lock file empty"))?
        .trim()
        .to_string();
    let ts = lines
        .next()
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(0);
    Ok((owner, ts))
}

fn write_lock(owner: &str, ts: i64) -> Result<()> {
    // Atomic-ish: write to .tmp then rename. Cheap because /tmp is
    // tmpfs.
    let tmp = format!("{}.tmp", LOCK_PATH);
    {
        let mut f = fs::File::create(&tmp)
            .with_context(|| format!("failed to create {}", tmp))?;
        writeln!(f, "{}", owner)?;
        writeln!(f, "{}", ts)?;
    }
    fs::rename(&tmp, LOCK_PATH)
        .with_context(|| format!("failed to rename {} -> {}", tmp, LOCK_PATH))?;
    Ok(())
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The lock path is global — tests that mutate it serialize via
    // this mutex to avoid cross-test interference.
    use std::sync::Mutex;
    static LOCK: Mutex<()> = Mutex::new(());

    fn with_tmp_lock<F: FnOnce()>(f: F) {
        let _g = LOCK.lock().unwrap();
        let _ = fs::remove_file(LOCK_PATH);
        f();
        let _ = fs::remove_file(LOCK_PATH);
    }

    #[test]
    fn acquire_when_unheld_succeeds() {
        with_tmp_lock(|| {
            assert!(try_acquire("telemetry").unwrap());
            assert_eq!(current_owner().as_deref(), Some("telemetry"));
        });
    }

    #[test]
    fn acquire_when_we_already_own_succeeds() {
        with_tmp_lock(|| {
            try_acquire("telemetry").unwrap();
            assert!(try_acquire("telemetry").unwrap(), "self-reacquire should succeed");
        });
    }

    #[test]
    fn acquire_when_other_owner_fresh_fails() {
        with_tmp_lock(|| {
            try_acquire("keep_awake").unwrap();
            assert!(!try_acquire("telemetry").unwrap());
            assert_eq!(current_owner().as_deref(), Some("keep_awake"));
        });
    }

    #[test]
    fn acquire_steals_stale_lock() {
        with_tmp_lock(|| {
            // Write a stale lock from another owner.
            write_lock("keep_awake", now_secs() - STALE_AFTER_SECS - 1).unwrap();
            assert!(try_acquire("telemetry").unwrap(), "stale lock should be stealable");
            assert_eq!(current_owner().as_deref(), Some("telemetry"));
        });
    }

    #[test]
    fn release_removes_when_we_own() {
        with_tmp_lock(|| {
            try_acquire("telemetry").unwrap();
            release("telemetry").unwrap();
            assert!(current_owner().is_none());
        });
    }

    #[test]
    fn release_noop_when_other_owns() {
        with_tmp_lock(|| {
            try_acquire("keep_awake").unwrap();
            release("telemetry").unwrap();
            assert_eq!(current_owner().as_deref(), Some("keep_awake"));
        });
    }
}
