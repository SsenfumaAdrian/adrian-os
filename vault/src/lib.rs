#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultError {
    OperationFailed,
}

pub struct SymmetricKey {
    bytes: [u8; KEY_LEN],
}

impl SymmetricKey {
    pub const fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self { bytes }
    }
}

pub fn encrypt(
    key: &SymmetricKey,
    nonce: &[u8; NONCE_LEN],
    associated_data: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, VaultError> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key.bytes));
    cipher
        .encrypt(
            Nonce::from_slice(nonce),
            Payload { msg: plaintext, aad: associated_data },
        )
        .map_err(|_| VaultError::OperationFailed)
}

pub fn decrypt(
    key: &SymmetricKey,
    nonce: &[u8; NONCE_LEN],
    associated_data: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, VaultError> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key.bytes));
    cipher
        .decrypt(
            Nonce::from_slice(nonce),
            Payload { msg: ciphertext, aad: associated_data },
        )
        .map_err(|_| VaultError::OperationFailed)
}

/// Derive keying material from `ikm` (input keying material) and
/// `salt`, using RFC 5869 HKDF-SHA256. Writes directly into `okm`
/// (output keying material) rather than returning a Vec -- unlike
/// encrypt/decrypt, HKDF's expand step naturally works against a
/// fixed-size caller buffer, so this needs no heap allocation at all.
/// `okm`'s length is however much keying material is actually needed
/// (32 bytes for one SymmetricKey, more if deriving several keys from
/// one input at once via `info` as a domain separator).
pub fn derive_key(salt: &[u8], ikm: &[u8], info: &[u8], okm: &mut [u8]) -> Result<(), VaultError> {
    let hk = Hkdf::<Sha256>::new(Some(salt), ikm);
    hk.expand(info, okm).map_err(|_| VaultError::OperationFailed)
}

pub const SIGNING_SEED_LEN: usize = 32;
pub const PUBLIC_KEY_LEN: usize = 32;
pub const SIGNATURE_LEN: usize = 64;

/// An Ed25519 signing key, built from a 32-byte seed -- matches RFC
/// 8032's own "private key" convention directly, so the RFC's test
/// vectors can be used as-is with no reinterpretation. Deliberately
/// no key-generation function here: Ed25519 key generation needs a
/// real random seed, and this kernel has no entropy source wired up
/// yet, same reasoning SymmetricKey and derive_key's nonce parameter
/// already carry -- the seed is the caller's problem to supply
/// safely, not something to generate badly and pretend is fine.
pub struct SigningKey {
    inner: ed25519_dalek::SigningKey,
}

impl SigningKey {
    pub fn from_seed(seed: &[u8; SIGNING_SEED_LEN]) -> Self {
        Self { inner: ed25519_dalek::SigningKey::from_bytes(seed) }
    }

    pub fn public_key(&self) -> [u8; PUBLIC_KEY_LEN] {
        self.inner.verifying_key().to_bytes()
    }

    pub fn sign(&self, message: &[u8]) -> [u8; SIGNATURE_LEN] {
        use ed25519_dalek::Signer;
        self.inner.sign(message).to_bytes()
    }
}

/// Verify `signature` over `message` under `public_key`. Ed25519
/// verification is deterministic and needs no secret state -- unlike
/// signing, there's no seed/entropy concern here at all.
pub fn verify(
    public_key: &[u8; PUBLIC_KEY_LEN],
    message: &[u8],
    signature: &[u8; SIGNATURE_LEN],
) -> Result<(), VaultError> {
    use ed25519_dalek::Verifier;
    let verifying_key =
        ed25519_dalek::VerifyingKey::from_bytes(public_key).map_err(|_| VaultError::OperationFailed)?;
    let sig = ed25519_dalek::Signature::from_bytes(signature);
    verifying_key.verify(message, &sig).map_err(|_| VaultError::OperationFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_recovers_the_original_plaintext() {
        let key = SymmetricKey::from_bytes([7u8; KEY_LEN]);
        let nonce = [1u8; NONCE_LEN];
        let plaintext = b"a message worth protecting";

        let ciphertext = encrypt(&key, &nonce, b"", plaintext).unwrap();
        let recovered = decrypt(&key, &nonce, b"", &ciphertext).unwrap();

        assert_eq!(recovered, plaintext);
        // Ciphertext must not equal plaintext -- confirms something
        // actually happened, not a no-op that "round trips" trivially.
        assert_ne!(ciphertext.as_slice(), plaintext);
    }

    #[test]
    fn associated_data_is_authenticated_alongside_the_ciphertext() {
        let key = SymmetricKey::from_bytes([3u8; KEY_LEN]);
        let nonce = [9u8; NONCE_LEN];
        let plaintext = b"payload";
        let aad = b"header, not encrypted but must not be tampered with";

        let ciphertext = encrypt(&key, &nonce, aad, plaintext).unwrap();
        assert_eq!(decrypt(&key, &nonce, aad, &ciphertext).unwrap(), plaintext);
    }

    #[test]
    fn decrypt_fails_if_ciphertext_is_tampered_with() {
        // The actual point of AEAD, not just encryption: modifying the
        // ciphertext after the fact must be detected, not silently
        // decrypted into garbage or (worse) accepted.
        let key = SymmetricKey::from_bytes([5u8; KEY_LEN]);
        let nonce = [2u8; NONCE_LEN];
        let mut ciphertext = encrypt(&key, &nonce, b"", b"authentic message").unwrap();

        let last = ciphertext.len() - 1;
        ciphertext[last] ^= 0xFF;

        assert_eq!(decrypt(&key, &nonce, b"", &ciphertext), Err(VaultError::OperationFailed));
    }

    #[test]
    fn decrypt_fails_if_associated_data_is_tampered_with() {
        // AAD is authenticated but not encrypted -- changing it after
        // the fact must still be caught, same as ciphertext tampering.
        let key = SymmetricKey::from_bytes([5u8; KEY_LEN]);
        let nonce = [2u8; NONCE_LEN];
        let ciphertext = encrypt(&key, &nonce, b"original aad", b"message").unwrap();

        assert_eq!(
            decrypt(&key, &nonce, b"different aad", &ciphertext),
            Err(VaultError::OperationFailed)
        );
    }

    #[test]
    fn decrypt_fails_with_the_wrong_key() {
        let nonce = [4u8; NONCE_LEN];
        let ciphertext =
            encrypt(&SymmetricKey::from_bytes([1u8; KEY_LEN]), &nonce, b"", b"secret").unwrap();

        let wrong_key = SymmetricKey::from_bytes([2u8; KEY_LEN]);
        assert_eq!(decrypt(&wrong_key, &nonce, b"", &ciphertext), Err(VaultError::OperationFailed));
    }

    #[test]
    fn decrypt_fails_with_the_wrong_nonce() {
        let key = SymmetricKey::from_bytes([6u8; KEY_LEN]);
        let ciphertext = encrypt(&key, &[1u8; NONCE_LEN], b"", b"secret").unwrap();

        assert_eq!(
            decrypt(&key, &[2u8; NONCE_LEN], b"", &ciphertext),
            Err(VaultError::OperationFailed)
        );
    }

    /// RFC 8439 SS2.8.2, the official ChaCha20-Poly1305 AEAD test
    /// vector -- fetched directly from rfc-editor.org rather than
    /// typed from memory, given how easy a single wrong hex digit
    /// would be to miss. This is the verification that actually
    /// matters for crypto code specifically: round-trip tests only
    /// prove self-consistency (a broken implementation could still
    /// round-trip with itself); this proves the wrapper produces
    /// output byte-for-byte identical to the standard algorithm,
    /// which is what interoperability with anything else actually
    /// speaking ChaCha20-Poly1305 depends on.
    #[test]
    fn matches_the_official_rfc_8439_test_vector() {
        let key = SymmetricKey::from_bytes([
            0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d,
            0x8e, 0x8f, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b,
            0x9c, 0x9d, 0x9e, 0x9f,
        ]);

        // nonce = 32-bit fixed-common part (07 00 00 00) | IV (40 41
        // 42 43 44 45 46 47), per the RFC's own pseudocode.
        let nonce: [u8; NONCE_LEN] =
            [0x07, 0x00, 0x00, 0x00, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47];

        let aad: [u8; 12] = [0x50, 0x51, 0x52, 0x53, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7];

        let plaintext: [u8; 114] = [
            0x4c, 0x61, 0x64, 0x69, 0x65, 0x73, 0x20, 0x61, 0x6e, 0x64, 0x20, 0x47, 0x65, 0x6e,
            0x74, 0x6c, 0x65, 0x6d, 0x65, 0x6e, 0x20, 0x6f, 0x66, 0x20, 0x74, 0x68, 0x65, 0x20,
            0x63, 0x6c, 0x61, 0x73, 0x73, 0x20, 0x6f, 0x66, 0x20, 0x27, 0x39, 0x39, 0x3a, 0x20,
            0x49, 0x66, 0x20, 0x49, 0x20, 0x63, 0x6f, 0x75, 0x6c, 0x64, 0x20, 0x6f, 0x66, 0x66,
            0x65, 0x72, 0x20, 0x79, 0x6f, 0x75, 0x20, 0x6f, 0x6e, 0x6c, 0x79, 0x20, 0x6f, 0x6e,
            0x65, 0x20, 0x74, 0x69, 0x70, 0x20, 0x66, 0x6f, 0x72, 0x20, 0x74, 0x68, 0x65, 0x20,
            0x66, 0x75, 0x74, 0x75, 0x72, 0x65, 0x2c, 0x20, 0x73, 0x75, 0x6e, 0x73, 0x63, 0x72,
            0x65, 0x65, 0x6e, 0x20, 0x77, 0x6f, 0x75, 0x6c, 0x64, 0x20, 0x62, 0x65, 0x20, 0x69,
            0x74, 0x2e,
        ];

        // Expected output = ciphertext (114 bytes) followed by the
        // 16-byte tag, matching how Aead::encrypt concatenates them.
        let expected_ciphertext_and_tag: [u8; 130] = [
            0xd3, 0x1a, 0x8d, 0x34, 0x64, 0x8e, 0x60, 0xdb, 0x7b, 0x86, 0xaf, 0xbc, 0x53, 0xef,
            0x7e, 0xc2, 0xa4, 0xad, 0xed, 0x51, 0x29, 0x6e, 0x08, 0xfe, 0xa9, 0xe2, 0xb5, 0xa7,
            0x36, 0xee, 0x62, 0xd6, 0x3d, 0xbe, 0xa4, 0x5e, 0x8c, 0xa9, 0x67, 0x12, 0x82, 0xfa,
            0xfb, 0x69, 0xda, 0x92, 0x72, 0x8b, 0x1a, 0x71, 0xde, 0x0a, 0x9e, 0x06, 0x0b, 0x29,
            0x05, 0xd6, 0xa5, 0xb6, 0x7e, 0xcd, 0x3b, 0x36, 0x92, 0xdd, 0xbd, 0x7f, 0x2d, 0x77,
            0x8b, 0x8c, 0x98, 0x03, 0xae, 0xe3, 0x28, 0x09, 0x1b, 0x58, 0xfa, 0xb3, 0x24, 0xe4,
            0xfa, 0xd6, 0x75, 0x94, 0x55, 0x85, 0x80, 0x8b, 0x48, 0x31, 0xd7, 0xbc, 0x3f, 0xf4,
            0xde, 0xf0, 0x8e, 0x4b, 0x7a, 0x9d, 0xe5, 0x76, 0xd2, 0x65, 0x86, 0xce, 0xc6, 0x4b,
            0x61, 0x16,
            // tag:
            0x1a, 0xe1, 0x0b, 0x59, 0x4f, 0x09, 0xe2, 0x6a, 0x7e, 0x90, 0x2e, 0xcb, 0xd0, 0x60,
            0x06, 0x91,
        ];

        let result = encrypt(&key, &nonce, &aad, &plaintext).unwrap();
        assert_eq!(result.as_slice(), &expected_ciphertext_and_tag[..]);

        // And the reverse direction: decrypting the RFC's own
        // ciphertext with the RFC's own inputs must recover exactly
        // the RFC's own plaintext.
        let recovered = decrypt(&key, &nonce, &aad, &expected_ciphertext_and_tag).unwrap();
        assert_eq!(recovered.as_slice(), &plaintext[..]);
    }

    #[test]
    fn derive_key_produces_deterministic_output() {
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        derive_key(b"salt", b"input key material", b"context", &mut a).unwrap();
        derive_key(b"salt", b"input key material", b"context", &mut b).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn derive_key_output_depends_on_every_input() {
        let base = {
            let mut out = [0u8; 32];
            derive_key(b"salt", b"ikm", b"info", &mut out).unwrap();
            out
        };

        let different_salt = {
            let mut out = [0u8; 32];
            derive_key(b"different-salt", b"ikm", b"info", &mut out).unwrap();
            out
        };
        let different_ikm = {
            let mut out = [0u8; 32];
            derive_key(b"salt", b"different-ikm", b"info", &mut out).unwrap();
            out
        };
        let different_info = {
            let mut out = [0u8; 32];
            derive_key(b"salt", b"ikm", b"different-info", &mut out).unwrap();
            out
        };

        // `info` acting as a real domain separator is the actual
        // point of including it -- deriving two different keys from
        // the same salt and ikm (e.g. one for encryption, one for
        // authentication) must not produce the same output.
        assert_ne!(base, different_salt);
        assert_ne!(base, different_ikm);
        assert_ne!(base, different_info);
    }

    /// RFC 5869 Appendix A.1, "Basic test case with SHA-256" --
    /// cross-checked against multiple independent sources (the RFC
    /// itself via rfc-editor.org, and the IETF datatracker mirror)
    /// before trusting the exact hex, same discipline as the AEAD
    /// vector above.
    #[test]
    fn matches_rfc_5869_test_case_1() {
        let ikm: [u8; 22] = [
            0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b,
            0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b,
        ];
        let salt: [u8; 13] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
        ];
        let info: [u8; 10] = [0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9];

        // L = 42
        let expected_okm: [u8; 42] = [
            0x3c, 0xb2, 0x5f, 0x25, 0xfa, 0xac, 0xd5, 0x7a, 0x90, 0x43, 0x4f, 0x64, 0xd0, 0x36,
            0x2f, 0x2a, 0x2d, 0x2d, 0x0a, 0x90, 0xcf, 0x1a, 0x5a, 0x4c, 0x5d, 0xb0, 0x2d, 0x56,
            0xec, 0xc4, 0xc5, 0xbf, 0x34, 0x00, 0x72, 0x08, 0xd5, 0xb8, 0x87, 0x18, 0x58, 0x65,
        ];

        let mut okm = [0u8; 42];
        derive_key(&salt, &ikm, &info, &mut okm).unwrap();
        assert_eq!(okm, expected_okm);
    }

    /// RFC 5869 Appendix A.3, "Test with SHA-256 and zero-length
    /// salt/info" -- the edge case where salt and info are both
    /// present-but-empty rather than omitted. Worth testing
    /// separately from Test Case 1: an implementation could pass the
    /// basic case while mishandling the empty-input edge case (e.g.
    /// treating a zero-length salt as "no salt" incorrectly, or
    /// panicking on an empty slice).
    #[test]
    fn matches_rfc_5869_test_case_3_empty_salt_and_info() {
        let ikm: [u8; 22] = [
            0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b,
            0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b,
        ];

        let expected_okm: [u8; 42] = [
            0x8d, 0xa4, 0xe7, 0x75, 0xa5, 0x63, 0xc1, 0x8f, 0x71, 0x5f, 0x80, 0x2a, 0x06, 0x3c,
            0x5a, 0x31, 0xb8, 0xa1, 0x1f, 0x5c, 0x5e, 0xe1, 0x87, 0x9e, 0xc3, 0x45, 0x4e, 0x5f,
            0x3c, 0x73, 0x8d, 0x2d, 0x9d, 0x20, 0x13, 0x95, 0xfa, 0xa4, 0xb6, 0x1a, 0x96, 0xc8,
        ];

        let mut okm = [0u8; 42];
        derive_key(&[], &ikm, &[], &mut okm).unwrap();
        assert_eq!(okm, expected_okm);
    }

    #[test]
    fn sign_then_verify_succeeds() {
        let key = SigningKey::from_seed(&[42u8; SIGNING_SEED_LEN]);
        let signature = key.sign(b"a message worth signing");
        assert!(verify(&key.public_key(), b"a message worth signing", &signature).is_ok());
    }

    #[test]
    fn verify_fails_if_the_message_is_altered() {
        let key = SigningKey::from_seed(&[7u8; SIGNING_SEED_LEN]);
        let signature = key.sign(b"original message");
        assert_eq!(
            verify(&key.public_key(), b"altered message", &signature),
            Err(VaultError::OperationFailed)
        );
    }

    #[test]
    fn verify_fails_with_the_wrong_public_key() {
        let key = SigningKey::from_seed(&[1u8; SIGNING_SEED_LEN]);
        let other_key = SigningKey::from_seed(&[2u8; SIGNING_SEED_LEN]);
        let signature = key.sign(b"message");
        assert_eq!(
            verify(&other_key.public_key(), b"message", &signature),
            Err(VaultError::OperationFailed)
        );
    }

    #[test]
    fn verify_fails_if_the_signature_is_tampered_with() {
        let key = SigningKey::from_seed(&[3u8; SIGNING_SEED_LEN]);
        let mut signature = key.sign(b"message");
        signature[0] ^= 0xFF;
        assert_eq!(
            verify(&key.public_key(), b"message", &signature),
            Err(VaultError::OperationFailed)
        );
    }

    /// RFC 8032 SS7.1, Test 1 -- the empty-message case. Corroborated
    /// across five independent sources (RFC Editor's own errata page
    /// quoting the RFC text, an IETF RFC mirror, a Linux kernel patch
    /// citing the same section, and two independent third-party
    /// programmatic verifications) all agreeing byte-for-byte before
    /// trusting the exact hex.
    #[test]
    fn matches_rfc_8032_test_1_empty_message() {
        let seed: [u8; SIGNING_SEED_LEN] = [
            0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec,
            0x2c, 0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03,
            0x1c, 0xae, 0x7f, 0x60,
        ];
        let expected_public_key: [u8; PUBLIC_KEY_LEN] = [
            0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64,
            0x07, 0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68,
            0xf7, 0x07, 0x51, 0x1a,
        ];
        let expected_signature: [u8; SIGNATURE_LEN] = [
            0xe5, 0x56, 0x43, 0x00, 0xc3, 0x60, 0xac, 0x72, 0x90, 0x86, 0xe2, 0xcc, 0x80, 0x6e,
            0x82, 0x8a, 0x84, 0x87, 0x7f, 0x1e, 0xb8, 0xe5, 0xd9, 0x74, 0xd8, 0x73, 0xe0, 0x65,
            0x22, 0x49, 0x01, 0x55, 0x5f, 0xb8, 0x82, 0x15, 0x90, 0xa3, 0x3b, 0xac, 0xc6, 0x1e,
            0x39, 0x70, 0x1c, 0xf9, 0xb4, 0x6b, 0xd2, 0x5b, 0xf5, 0xf0, 0x59, 0x5b, 0xbe, 0x24,
            0x65, 0x51, 0x41, 0x43, 0x8e, 0x7a, 0x10, 0x0b,
        ];

        let key = SigningKey::from_seed(&seed);
        // The derived public key must match the RFC's stated public
        // key too, not just the signature -- an independent check
        // beyond the signature bytes alone.
        assert_eq!(key.public_key(), expected_public_key);

        let signature = key.sign(&[]);
        assert_eq!(signature, expected_signature);
        assert!(verify(&expected_public_key, &[], &expected_signature).is_ok());
    }

    /// RFC 8032 SS7.1, Test 2 -- a non-empty (1-byte) message, the
    /// same corroboration standard as Test 1 above.
    #[test]
    fn matches_rfc_8032_test_2_one_byte_message() {
        let seed: [u8; SIGNING_SEED_LEN] = [
            0x4c, 0xcd, 0x08, 0x9b, 0x28, 0xff, 0x96, 0xda, 0x9d, 0xb6, 0xc3, 0x46, 0xec, 0x11,
            0x4e, 0x0f, 0x5b, 0x8a, 0x31, 0x9f, 0x35, 0xab, 0xa6, 0x24, 0xda, 0x8c, 0xf6, 0xed,
            0x4f, 0xb8, 0xa6, 0xfb,
        ];
        let expected_public_key: [u8; PUBLIC_KEY_LEN] = [
            0x3d, 0x40, 0x17, 0xc3, 0xe8, 0x43, 0x89, 0x5a, 0x92, 0xb7, 0x0a, 0xa7, 0x4d, 0x1b,
            0x7e, 0xbc, 0x9c, 0x98, 0x2c, 0xcf, 0x2e, 0xc4, 0x96, 0x8c, 0xc0, 0xcd, 0x55, 0xf1,
            0x2a, 0xf4, 0x66, 0x0c,
        ];
        let message: [u8; 1] = [0x72];
        let expected_signature: [u8; SIGNATURE_LEN] = [
            0x92, 0xa0, 0x09, 0xa9, 0xf0, 0xd4, 0xca, 0xb8, 0x72, 0x0e, 0x82, 0x0b, 0x5f, 0x64,
            0x25, 0x40, 0xa2, 0xb2, 0x7b, 0x54, 0x16, 0x50, 0x3f, 0x8f, 0xb3, 0x76, 0x22, 0x23,
            0xeb, 0xdb, 0x69, 0xda, 0x08, 0x5a, 0xc1, 0xe4, 0x3e, 0x15, 0x99, 0x6e, 0x45, 0x8f,
            0x36, 0x13, 0xd0, 0xf1, 0x1d, 0x8c, 0x38, 0x7b, 0x2e, 0xae, 0xb4, 0x30, 0x2a, 0xee,
            0xb0, 0x0d, 0x29, 0x16, 0x12, 0xbb, 0x0c, 0x00,
        ];

        let key = SigningKey::from_seed(&seed);
        assert_eq!(key.public_key(), expected_public_key);

        let signature = key.sign(&message);
        assert_eq!(signature, expected_signature);
        assert!(verify(&expected_public_key, &message, &expected_signature).is_ok());
    }
}
