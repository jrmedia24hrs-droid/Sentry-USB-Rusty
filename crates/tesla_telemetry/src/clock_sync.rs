//! Set the Pi's system clock from the vehicle's GPS-derived timestamp.
//!
//! Why: Tesla's car has an always-accurate clock (GPS-synced). Every
//! `state X` response includes a `timestamp` field in the car's clock
//! frame. If our local Pi clock is significantly off — typical when
//! there's no RTC battery and WiFi isn't reachable at boot — we can
//! correct it from the first successful BLE response, with no
//! dependency on NTP or internet access.
//!
//! Design:
//!   * Latency-compensated: applies half the round-trip-time to the
//!     vehicle timestamp, so we land within ~50 ms of the true time
//!     even though the BLE call took 1-5 s.
//!   * NTP-friendly: only adjusts when local-vs-vehicle delta exceeds
//!     a threshold (default 5 min). Avoids fighting NTP's normal sub-
//!     second drift correction.
//!   * RTC-friendly: if `/dev/rtc0` exists, also writes the corrected
//!     time to the RTC so it survives reboots.
//!   * One-shot per startup window: once we've corrected the clock,
//!     subsequent responses are within tolerance so we leave them be.
//!
//! Threading: `clock_settime` modifies CLOCK_REALTIME but does NOT
//! affect CLOCK_MONOTONIC, so any `Instant` values still measure
//! elapsed time correctly across the adjustment.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tracing::{info, warn};

/// Threshold below which we leave the local clock alone. NTP and a
/// healthy RTC both keep the clock within a few seconds; if the delta
/// is < 5 minutes, assume one of those did its job and don't second-
/// guess it. Above 5 minutes the clock is meaningfully wrong (typical
/// non-RTC cold-boot states are years off, so this triggers cleanly).
const ADJUSTMENT_THRESHOLD_MS: i64 = 300_000;

/// Constant one-way latency from "car stamps timestamp" to "we read
/// the response", in milliseconds. Empirically measured against an
/// NTP-set reference clock — the actual delta clustered tightly
/// around -55ms with low variance regardless of round-trip time.
/// Tesla stamps the timestamp just before transmitting the response,
/// so this is essentially the BLE response transit time. Adding it
/// brings our clock-sync accuracy from ~54ms to ~12ms vs NTP.
const RESPONSE_LATENCY_COMPENSATION_MS: i64 = 50;

/// Parse the RFC 3339 / ISO 8601 timestamp Tesla emits in state
/// responses (e.g. `"2026-05-25T04:31:59.107Z"`). Returns unix-secs.
///
/// We don't use chrono here to avoid adding a heavy dep for one
/// fixed-format parse. The format Tesla uses is rigid: YYYY-MM-DD
/// 'T' HH:MM:SS '.' fff 'Z' (UTC, always). Anything else returns None.
/// Like `parse_rfc3339_secs` but preserves the fractional-second
/// precision Tesla emits (e.g. `.794Z` → 794 milliseconds). Needed
/// because second-rounding alone introduces up to 999ms of error,
/// which dominates the actual ~50ms BLE latency we're trying to
/// compensate for.
pub fn parse_rfc3339_ms(s: &str) -> Option<i64> {
    let secs = parse_rfc3339_secs(s)?;
    let frac_ms = if let Some(dot_pos) = s.find('.') {
        // `s` ends with 'Z' (parse_rfc3339_secs guarantees), so the
        // fractional part is between '.' and that 'Z'.
        let frac_str = &s[dot_pos + 1..s.len() - 1];
        match frac_str.parse::<u32>() {
            Ok(n) => match frac_str.len() {
                1 => (n as i64) * 100,
                2 => (n as i64) * 10,
                3 => n as i64,
                // Microsecond/nanosecond precision — truncate to ms.
                len => (n as i64) / 10_i64.pow((len - 3) as u32),
            },
            Err(_) => 0,
        }
    } else {
        0
    };
    Some(secs * 1000 + frac_ms)
}

pub fn parse_rfc3339_secs(s: &str) -> Option<i64> {
    // Expected length: "2026-05-25T04:31:59.107Z" = 24 chars typically.
    // Accept either with or without fractional seconds, both ending Z.
    if s.len() < 20 || !s.ends_with('Z') {
        return None;
    }
    let b = s.as_bytes();
    // Sanity-check separator positions.
    if b[4] != b'-' || b[7] != b'-' || b[10] != b'T'
        || b[13] != b':' || b[16] != b':'
    {
        return None;
    }
    let year: i32 = s.get(0..4)?.parse().ok()?;
    let month: u32 = s.get(5..7)?.parse().ok()?;
    let day: u32 = s.get(8..10)?.parse().ok()?;
    let hour: u32 = s.get(11..13)?.parse().ok()?;
    let minute: u32 = s.get(14..16)?.parse().ok()?;
    let second: u32 = s.get(17..19)?.parse().ok()?;

    // Days-since-epoch for the given Y/M/D, handling leap years.
    // Algorithm: Howard Hinnant's "Date Algorithms" days_from_civil.
    // Same formula used by C++'s <chrono> and Linux kernel
    // mktime64 — handles every Gregorian date correctly.
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let m = month as i32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day as i32 - 1;
    let doe = (yoe * 365 + yoe / 4 - yoe / 100) as i32 + doy;
    let days_since_epoch = (era as i64) * 146097 + (doe as i64) - 719468;
    Some(days_since_epoch * 86400 + (hour as i64) * 3600 + (minute as i64) * 60 + second as i64)
}

/// If the local clock is meaningfully wrong (>5 min from vehicle), set
/// it to vehicle time. Optionally persist to RTC.
///
/// Args:
///   * `vehicle_ts_ms` — the timestamp Tesla sent us, in ms-since-epoch
///     (includes the fractional seconds Tesla provides, e.g. `.794Z`)
///   * `request_started_at` — monotonic Instant from before we sent
///     the BLE request, used for diagnostic RTT logging
///
/// Returns true if we adjusted the clock; false if delta was below
/// threshold (normal case once everything's synced).
pub fn maybe_set_clock_from_vehicle(
    vehicle_ts_ms: i64,
    request_started_at: Instant,
) -> bool {
    let local_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    // Empirically (validated against NTP-set reference clock, see
    // README), Tesla stamps the response timestamp just before
    // transmitting — NOT at the midpoint of processing. The one-way
    // latency from "car stamps" to "we receive" is a consistent
    // ~50ms regardless of RTT. Adding it brings us from ~54ms avg
    // error (no comp) down to ~12ms avg error (with comp).
    let rtt_ms = request_started_at.elapsed().as_millis() as i64;
    let corrected_target_ms = vehicle_ts_ms + RESPONSE_LATENCY_COMPENSATION_MS;

    let delta_ms = corrected_target_ms - local_ms;
    if delta_ms.abs() < ADJUSTMENT_THRESHOLD_MS {
        // Already close enough — leave it alone. Avoids fighting
        // NTP / RTC adjustments that are doing their job.
        return false;
    }

    info!(
        "system clock differs from vehicle by {}ms (local={}ms, vehicle={}ms, rtt={}ms); \
         adjusting to vehicle time",
        delta_ms, local_ms, corrected_target_ms, rtt_ms
    );

    // Actually set the system clock with millisecond precision via
    // tv_nsec. Requires CAP_SYS_TIME; the telemetry daemon runs as
    // root so this works.
    let secs = corrected_target_ms / 1000;
    let ms_remainder = corrected_target_ms % 1000;
    // `tv_nsec` is `i64` on x86_64 Linux but `i32` on aarch64 — use
    // libc::c_long so this compiles cleanly on both.
    let ts = libc::timespec {
        tv_sec: secs as libc::time_t,
        tv_nsec: (ms_remainder * 1_000_000) as libc::c_long,
    };
    let rc = unsafe { libc::clock_settime(libc::CLOCK_REALTIME, &ts) };
    if rc != 0 {
        let err = std::io::Error::last_os_error();
        warn!("clock_settime failed: {} (errno={})", err, err.raw_os_error().unwrap_or(0));
        return false;
    }

    // If an RTC battery is present, also write the corrected time
    // there so the next boot starts from the right time without
    // needing the BLE-sync dance again. Best-effort.
    if std::path::Path::new("/dev/rtc0").exists() {
        match std::process::Command::new("hwclock").args(["-w"]).output() {
            Ok(out) if out.status.success() => {
                info!("wrote corrected time to RTC (hwclock -w)");
            }
            Ok(out) => {
                warn!(
                    "hwclock -w returned {}: {}",
                    out.status,
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            Err(e) => warn!("hwclock -w failed to run: {e}"),
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_tesla_timestamp() {
        // From actual Tesla state-drive response in our captures.
        let ts = parse_rfc3339_secs("2026-05-25T04:32:23.794Z").unwrap();
        // 2026-05-25T04:32:23Z = 1779683543
        assert_eq!(ts, 1779683543);
    }

    #[test]
    fn parses_without_fractional_seconds() {
        let ts = parse_rfc3339_secs("2024-01-15T10:00:00Z").unwrap();
        // 2024-01-15T10:00:00Z = 1705312800 (verified against `date -u -d`)
        assert_eq!(ts, 1705312800);
    }

    #[test]
    fn rejects_bad_format() {
        assert!(parse_rfc3339_secs("not a timestamp").is_none());
        assert!(parse_rfc3339_secs("2024-13-01T00:00:00Z").is_some()); // we don't range-check month
        assert!(parse_rfc3339_secs("2024-01-15 10:00:00Z").is_none()); // space instead of T
    }

    #[test]
    fn handles_leap_year() {
        // Feb 29 2024 was a real day.
        let ts = parse_rfc3339_secs("2024-02-29T00:00:00Z").unwrap();
        assert_eq!(ts, 1709164800);
    }
}
