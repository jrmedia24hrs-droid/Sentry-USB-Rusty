//! Load the user's Tesla BLE NIST P-256 private key + derive the
//! public key for SessionInfoRequest.

use std::path::Path;

use anyhow::{Context, Result, bail};
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
    /// Read a Tesla BLE key file. Accepts both SEC1 PEM
    /// (`-----BEGIN EC PRIVATE KEY-----`, which is what tesla-keygen
    /// produces) and PKCS#8 PEM (`-----BEGIN PRIVATE KEY-----`).
    pub fn load(path: &Path) -> Result<Self> {
        let pem_str = std::fs::read_to_string(path)
            .with_context(|| format!("reading key file {}", path.display()))?;
        let parsed = pem::parse(&pem_str).context("parsing PEM envelope")?;
        let secret = match parsed.tag() {
            "EC PRIVATE KEY" => secret_from_sec1_der(parsed.contents())
                .context("parsing SEC1 DER EC private key")?,
            "PRIVATE KEY" => SecretKey::from_pkcs8_der(parsed.contents())
                .context("parsing PKCS#8 DER private key")?,
            other => bail!(
                "unexpected PEM type label {:?}; expected 'EC PRIVATE KEY' or 'PRIVATE KEY'",
                other
            ),
        };
        let pub_uncompressed = secret.public_key().to_sec1_bytes().as_ref().to_vec();
        Ok(Self {
            secret,
            pub_uncompressed,
        })
    }
}

/// Hand-parse SEC1 ECPrivateKey DER to extract the 32-byte scalar.
/// p256 0.13 doesn't expose `from_sec1_pem`/`from_sec1_der` directly
/// under the feature set we use, so we walk the small fixed-shape
/// ASN.1 ourselves.
///
/// SEC1 layout (RFC 5915):
///   SEQUENCE {
///     INTEGER 1                              // version
///     OCTET STRING (32 bytes)                // privateKey
///     [0] OID 1.2.840.10045.3.1.7  OPTIONAL  // P-256 curve
///     [1] BIT STRING (uncompressed pubkey) OPTIONAL
///   }
fn secret_from_sec1_der(der: &[u8]) -> Result<SecretKey> {
    let mut i = 0;
    // Expect SEQUENCE
    if der.get(i) != Some(&0x30) {
        bail!("SEC1: expected SEQUENCE at offset 0");
    }
    i += 1;
    // Skip length bytes. ASN.1 length: if high bit set on first byte,
    // low bits are the count of further length bytes (we don't actually
    // care about the value, just how many to skip).
    let first_len = der.get(i).copied().context("SEC1: truncated length")?;
    if first_len & 0x80 == 0 {
        i += 1;
    } else {
        i += 1 + (first_len & 0x7f) as usize;
    }
    // Expect INTEGER 1 (`02 01 01`)
    if der.get(i..i + 3) != Some(&[0x02, 0x01, 0x01]) {
        bail!("SEC1: expected INTEGER version 1 at offset {}", i);
    }
    i += 3;
    // Expect OCTET STRING length 32 (`04 20`)
    if der.get(i..i + 2) != Some(&[0x04, 0x20]) {
        bail!("SEC1: expected 32-byte OCTET STRING at offset {}", i);
    }
    i += 2;
    let scalar = der
        .get(i..i + 32)
        .context("SEC1: truncated private key bytes")?;
    SecretKey::from_slice(scalar).context("invalid P-256 scalar")
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::SecretKey;
    use p256::elliptic_curve::rand_core::OsRng;
    use p256::pkcs8::EncodePrivateKey;

    #[test]
    fn round_trip_generated_pkcs8_key() {
        let key = SecretKey::random(&mut OsRng);
        let pem = key.to_pkcs8_pem(p256::pkcs8::LineEnding::LF).unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), pem.as_bytes()).unwrap();

        let loaded = KeyPair::load(tmp.path()).unwrap();
        assert_eq!(loaded.pub_uncompressed.len(), 65);
        assert_eq!(loaded.pub_uncompressed[0], 0x04);
    }

    #[test]
    fn parses_sec1_pem_from_openssl() {
        // SEC1 PEM equivalent to the format `tesla-keygen` produces.
        // Generated via:
        //   openssl ecparam -name prime256v1 -genkey -noout
        // The exact bytes don't matter — just that the SEC1 path works.
        let pem = "-----BEGIN EC PRIVATE KEY-----\n\
                   MHcCAQEEIBnEX3tDgQHQX5IcAOA2RrvHV7ZzNeb7BLJ3vh7zVRpJoAoGCCqGSM49\n\
                   AwEHoUQDQgAEpUEnGcbqLEKMRwH69lcLN1H3xR/Mp3CY+QhBZkS1eOPF8Pdvkk0Q\n\
                   jiNAS/lZJaufnRu3WSjNu5xAvI4lNYjPiQ==\n\
                   -----END EC PRIVATE KEY-----\n";
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), pem).unwrap();

        let loaded = KeyPair::load(tmp.path()).unwrap();
        assert_eq!(loaded.pub_uncompressed.len(), 65);
        assert_eq!(loaded.pub_uncompressed[0], 0x04);
    }
}
