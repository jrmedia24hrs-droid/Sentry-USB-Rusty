//! Derive a Tesla's advertised BLE local name from its VIN.
//!
//! Tesla cars advertise as `S<16-hex>C` where the hex is the
//! lowercase representation of the first 8 bytes of SHA1(VIN).

use sha1::{Digest, Sha1};

/// Compute the BLE local name Tesla advertises for the given VIN.
/// Returns 18-char string: `S` + 16 hex chars + `C`.
pub fn vehicle_local_name(vin: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(vin.as_bytes());
    let digest = hasher.finalize();
    format!("S{}C", hex::encode(&digest[..8]))
}

/// True when `name` looks like a Tesla local name format we'd expect
/// from any car. Used as a cheap pre-filter when scanning many
/// BLE devices.
pub fn looks_like_tesla_name(name: &str) -> bool {
    name.len() == 18
        && name.starts_with('S')
        && name.ends_with('C')
        && name[1..17].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_sha1_of_input() {
        // Synthetic test fixture. Verify externally with:
        //   echo -n "1FAKEVIN000000001" | shasum -a 1 | cut -c1-16
        let name = vehicle_local_name("1FAKEVIN000000001");
        assert_eq!(name, "Sf116f5512678aa28C");
    }

    #[test]
    fn length_is_always_18() {
        let name = vehicle_local_name("12345678901234567");
        assert_eq!(name.len(), 18);
    }

    #[test]
    fn shape_check() {
        assert!(looks_like_tesla_name("S0123456789abcdefC"));
        assert!(!looks_like_tesla_name("Tesla"));
        assert!(!looks_like_tesla_name("S0123456789abcdef")); // no suffix
        assert!(!looks_like_tesla_name("X0123456789abcdefC")); // wrong prefix
        assert!(!looks_like_tesla_name("S0123456789abcdegC")); // non-hex
    }
}
