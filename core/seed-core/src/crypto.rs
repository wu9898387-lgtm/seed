use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

pub const ED25519_PUBLIC_KEY_LEN: usize = 32;
pub const ED25519_SECRET_KEY_LEN: usize = 32;
pub const ED25519_SIGNATURE_LEN: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignatureError {
    InvalidPublicKey,
    VerificationFailed,
}

/// Event-signing key wrapper for the Phase 0 Ed25519 spike.
///
/// Secure key generation and platform key storage are intentionally separate
/// concerns. This type never exposes private-key bytes after construction.
pub struct EventSigningKey(SigningKey);

impl EventSigningKey {
    pub fn from_secret_bytes(secret: &[u8; ED25519_SECRET_KEY_LEN]) -> Self {
        Self(SigningKey::from_bytes(secret))
    }

    pub fn verifying_key_bytes(&self) -> [u8; ED25519_PUBLIC_KEY_LEN] {
        self.0.verifying_key().to_bytes()
    }

    pub fn sign(&self, message: &[u8]) -> [u8; ED25519_SIGNATURE_LEN] {
        self.0.sign(message).to_bytes()
    }
}

pub fn verify_event_signature(
    public_key: &[u8; ED25519_PUBLIC_KEY_LEN],
    message: &[u8],
    signature: &[u8; ED25519_SIGNATURE_LEN],
) -> Result<(), SignatureError> {
    let verifying_key =
        VerifyingKey::from_bytes(public_key).map_err(|_| SignatureError::InvalidPublicKey)?;
    let signature = Signature::from_bytes(signature);

    verifying_key
        .verify_strict(message, &signature)
        .map_err(|_| SignatureError::VerificationFailed)
}

#[cfg(test)]
mod tests {
    use super::{verify_event_signature, EventSigningKey};

    const MESSAGE: &[u8] = b"seed:phase0:event-signature:v1";

    #[test]
    fn signs_verifies_and_rejects_tampering() {
        let secret = [0x11; 32];
        let signing_key = EventSigningKey::from_secret_bytes(&secret);
        let public_key = signing_key.verifying_key_bytes();
        let signature = signing_key.sign(MESSAGE);

        let expected_public_key = [
            0xd0, 0x4a, 0xb2, 0x32, 0x74, 0x2b, 0xb4, 0xab, 0x3a, 0x13, 0x68, 0xbd, 0x46, 0x15,
            0xe4, 0xe6, 0xd0, 0x22, 0x4a, 0xb7, 0x1a, 0x01, 0x6b, 0xaf, 0x85, 0x20, 0xa3, 0x32,
            0xc9, 0x77, 0x87, 0x37,
        ];
        let expected_signature = [
            0x0b, 0x0e, 0x49, 0x1a, 0x61, 0xde, 0x67, 0xf3, 0x90, 0x93, 0x83, 0x60, 0x8b, 0xb4,
            0xb1, 0xf7, 0xed, 0xc2, 0x16, 0xe9, 0x51, 0xb3, 0xa3, 0x64, 0x27, 0xa7, 0x27, 0x6e,
            0x6d, 0x83, 0x3e, 0x9b, 0x92, 0xbe, 0xdc, 0x7e, 0xa5, 0xb6, 0x8f, 0xe8, 0x2f, 0x25,
            0xba, 0x2c, 0x93, 0x06, 0x92, 0x49, 0x00, 0x91, 0xf1, 0x93, 0x7a, 0xd4, 0x6d, 0x84,
            0xe8, 0xda, 0x80, 0x3c, 0xae, 0x6c, 0xfb, 0x00,
        ];

        assert_eq!(public_key, expected_public_key);
        assert_eq!(signature, expected_signature);
        assert!(verify_event_signature(&public_key, MESSAGE, &signature).is_ok());

        let tampered = b"Seed:phase0:event-signature:v1";
        assert!(verify_event_signature(&public_key, tampered, &signature).is_err());
    }
}
