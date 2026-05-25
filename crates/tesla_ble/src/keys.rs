//! Load the user's Tesla BLE NIST P-256 private key + derive the
//! public key for SessionInfoRequest.

use std::path::Path;

use anyhow::{Context, Result};
use p256::SecretKey;
use p256::pkcs8::DecodePrivateKey;

/// Loaded ECDH keypair. The private key is for signing/ECDH; the
/// `pub_uncompressed` bytes are the 65-byte SEC1 format Tesla expects
/// in SessionInfoRequest (`0x04 || X || Y`).
pub struct KeyPair {
    pub secret: SecretKey,
    pub pub_uncompressed: Vec<u8>,
}

impl KeyPair {
    /// Read a Tesla BLE key file (PEM-encoded PKCS#8 NIST P-256).
    pub fn load(path: &Path) -> Result<Self> {
        let pem = std::fs::read_to_string(path)
            .with_context(|| format!("reading key file {}", path.display()))?;
        let secret = SecretKey::from_pkcs8_pem(&pem)
            .context("parsing PKCS#8 PEM private key (expected NIST P-256)")?;
        let pub_uncompressed = secret
            .public_key()
            .to_sec1_bytes()
            .as_ref()
            .to_vec();
        Ok(Self {
            secret,
            pub_uncompressed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::SecretKey;
    use p256::elliptic_curve::rand_core::OsRng;
    use p256::pkcs8::EncodePrivateKey;

    #[test]
    fn round_trip_generated_key() {
        // Generate a fresh key, encode as PEM, load it back, verify
        // the public key bytes are 65 bytes starting with 0x04.
        let key = SecretKey::random(&mut OsRng);
        let pem = key.to_pkcs8_pem(p256::pkcs8::LineEnding::LF).unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), pem.as_bytes()).unwrap();

        let loaded = KeyPair::load(tmp.path()).unwrap();
        assert_eq!(loaded.pub_uncompressed.len(), 65);
        assert_eq!(loaded.pub_uncompressed[0], 0x04);
    }
}
