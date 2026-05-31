use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use minisign_verify::{PublicKey, Signature};
use sha2::{Digest, Sha256};

use crate::Error;

/// Verify that the SHA-256 digest of `data` matches `expected_hex`.
///
/// The comparison is case-insensitive. Returns `false` if the hex string is
/// malformed or the digest does not match.
pub fn verify_sha256(data: &[u8], expected_hex: &str) -> bool {
    let digest = Sha256::digest(data);
    hex::encode(digest).eq_ignore_ascii_case(expected_hex)
}

/// Verify a minisign signature over `data`.
///
/// Both `signature_b64` and `pubkey_b64` are expected in the Tauri-encoded
/// format: the raw minisign file content (including the `untrusted comment:`
/// header line) base64-encoded into a single string — exactly as stored in
/// the `signature` field of [`AndroidManifest`] and in the Tauri update
/// key configuration.
pub fn verify_minisign(data: &[u8], signature_b64: &str, pubkey_b64: &str) -> Result<(), Error> {
    // Decode base64 → raw minisign file bytes → UTF-8 string
    let pk_bytes = BASE64
        .decode(pubkey_b64.trim())
        .map_err(|_| Error::Verify("invalid base64 in public key".into()))?;
    let pk_str = std::str::from_utf8(&pk_bytes)
        .map_err(|_| Error::Verify("public key is not valid UTF-8".into()))?;

    let sig_bytes = BASE64
        .decode(signature_b64.trim())
        .map_err(|_| Error::Verify("invalid base64 in signature".into()))?;
    let sig_str = std::str::from_utf8(&sig_bytes)
        .map_err(|_| Error::Verify("signature is not valid UTF-8".into()))?;

    let pk =
        PublicKey::decode(pk_str).map_err(|e| Error::Verify(format!("invalid public key: {e}")))?;
    let sig =
        Signature::decode(sig_str).map_err(|e| Error::Verify(format!("invalid signature: {e}")))?;

    pk.verify(data, &sig, false)
        .map_err(|_| Error::Verify("signature mismatch".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Throwaway keypair generated with `pnpm tauri signer generate -p "" ...`
    // Payload: b"notchkeep-test-payload"
    // These are safe to embed — the private key is not included.
    const TEST_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDYxQ0IzNDQ0ODU0RjJDMEMKUldRTUxFK0ZSRFRMWVVCbXV5WENKZ2h1Nm12YWQ4T0lFQzFOMFB4dUxuNlloNDZVK3BEVVphMnkK";
    const TEST_SIG: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVRTUxFK0ZSRFRMWWFSUlMzdEZUR3lCaXB1aTQrVTg0c2tQa0ZjaHVwL3c3b0o1M2NoRnRjOUdkT1l1Nlh0SXl6NXRJMjBCYXVNSm11K2g1RUZPUkNNc2Z1bzNXWFF4UGd3PQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzgwMjQ4NTU2CWZpbGU6YXBrdmVyaWZ5LmJpbgpHYlRkckxta0x6QVRmeHlHenYvUHJkTk5Vd2JiNUE0aElTV0JlYnN6a0Rlb0RjNkdzM1dEbzlSN3lvRjJNWGFQeUtmd2ZKSS9GTTFTTnJaL29WV3lCdz09Cg==";
    const PAYLOAD: &[u8] = b"notchkeep-test-payload";

    #[test]
    fn sha256_matches_and_rejects() {
        let data = b"hello";
        // echo -n "hello" | sha256sum → 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        assert!(
            verify_sha256(
                data,
                "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
            ),
            "correct hash should match"
        );
        assert!(!verify_sha256(data, "00"), "wrong hash should not match");
    }

    #[test]
    fn minisign_accepts_valid_and_rejects_tampered() {
        assert!(
            verify_minisign(PAYLOAD, TEST_SIG, TEST_PUBKEY).is_ok(),
            "valid signature should verify"
        );
        assert!(
            verify_minisign(b"tampered", TEST_SIG, TEST_PUBKEY).is_err(),
            "tampered payload should fail verification"
        );
    }
}
