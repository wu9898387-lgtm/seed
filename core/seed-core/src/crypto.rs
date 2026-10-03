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

        assert!(verify_event_signature(&public_key, MESSAGE, &signature).is_ok());

        let mut tampered = *MESSAGE;
        tampered[0] ^= 1;
        assert!(verify_event_signature(&public_key, &tampered, &signature).is_err());
    }
}
