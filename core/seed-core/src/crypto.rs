use crate::id::IdentityId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature(Vec<u8>);

impl Signature {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CryptoError {
    InvalidSignature,
    InvalidKey,
    BackendFailure,
}

/// Signing is provided by a dedicated crypto/key-storage adapter.
///
/// The core kernel does not expose private-key bytes through this interface.
pub trait Signer {
    fn sign(&self, message: &[u8]) -> Result<Signature, CryptoError>;
}

pub trait Verifier {
    fn verify(
        &self,
        author: &IdentityId,
        message: &[u8],
        signature: &Signature,
    ) -> Result<(), CryptoError>;
}
