#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};

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
}
