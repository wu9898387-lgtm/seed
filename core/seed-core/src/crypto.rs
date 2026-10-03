use core::fmt;

use ed25519_dalek::{Signature as DalekSignature, Signer as _, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

pub const PUBLIC_KEY_LENGTH: usize = 32;
pub const SIGNATURE_LENGTH: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PublicKey([u8; PUBLIC_KEY_LENGTH]);

impl PublicKey {
    pub const fn from_bytes(bytes: [u8; PUBLIC_KEY_LENGTH]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; PUBLIC_KEY_LENGTH] {
        &self.0
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey(")?;
        for byte in &self.0[..4] {
            write!(f, "{byte:02x}")?;
        }
        write!(f, "…)")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Signature([u8; SIGNATURE_LENGTH]);

impl Signature {
    pub const fn from_bytes(bytes: [u8; SIGNATURE_LENGTH]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; SIGNATURE_LENGTH] {
        &self.0
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature(")?;
        for byte in &self.0[..4] {
            write!(f, "{byte:02x}")?;
        }
        write!(f, "…)")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CryptoError {
    EntropyUnavailable,
    InvalidSignature,
    InvalidPublicKey,
}

/// In-memory Ed25519 signer used by the Phase-1 identity/device kernel.
///
/// Private key bytes are deliberately not exposed by this API. Persistent
/// key storage or HSM adapters will sit behind a separate boundary later.
pub struct Ed25519Signer {
    key: SigningKey,
}

impl fmt::Debug for Ed25519Signer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Ed25519Signer([redacted])")
    }
}

impl Ed25519Signer {
    pub fn generate() -> Result<Self, CryptoError> {
        let mut secret = [0u8; 32];
        getrandom::fill(&mut secret).map_err(|_| CryptoError::EntropyUnavailable)?;
        Ok(Self::from_secret_bytes(secret))
    }

    /// Import secret seed material without providing a matching export path.
    ///
    /// The input copy is securely zeroized after the signing key is constructed.
    pub fn from_secret_bytes(mut secret: [u8; 32]) -> Self {
        let key = SigningKey::from_bytes(&secret);
        secret.zeroize();
        Self { key }
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey::from_bytes(self.key.verifying_key().to_bytes())
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        Signature::from_bytes(self.key.sign(message).to_bytes())
    }
}

/// Strict Ed25519 verification.
///
/// This rejects weak public keys and avoids compatibility-oriented validation.
pub fn verify_strict(
    public_key: &PublicKey,
    message: &[u8],
    signature: &Signature,
) -> Result<(), CryptoError> {
    let verifying_key = VerifyingKey::from_bytes(public_key.as_bytes())
        .map_err(|_| CryptoError::InvalidPublicKey)?;
    let signature = DalekSignature::try_from(&signature.as_bytes()[..])
        .map_err(|_| CryptoError::InvalidSignature)?;

    verifying_key
        .verify_strict(message, &signature)
        .map_err(|_| CryptoError::InvalidSignature)
}

pub(crate) fn hash32(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for part in parts {
        hasher.update(part);
    }

    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_round_trip_works() {
        let signer = Ed25519Signer::generate().unwrap();
        let message = b"seed crypto smoke";
        let signature = signer.sign(message);

        verify_strict(&signer.public_key(), message, &signature).unwrap();
    }

    #[test]
    fn altered_message_is_rejected() {
        let signer = Ed25519Signer::generate().unwrap();
        let signature = signer.sign(b"one");

        assert_eq!(
            verify_strict(&signer.public_key(), b"two", &signature),
            Err(CryptoError::InvalidSignature)
        );
    }

    #[test]
    fn secret_import_is_deterministic() {
        let a = Ed25519Signer::from_secret_bytes([7u8; 32]);
        let b = Ed25519Signer::from_secret_bytes([7u8; 32]);

        assert_eq!(a.public_key(), b.public_key());
        assert_eq!(a.sign(b"seed").as_bytes(), b.sign(b"seed").as_bytes());
    }
}
