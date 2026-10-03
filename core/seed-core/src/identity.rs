/// Opaque cryptographic identity identifier.
///
/// Phase 0 does not define how these bytes are derived. Key generation,
/// signatures, fingerprints, rotation, and recovery remain behind the future
/// crypto/identity boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct IdentityId([u8; Self::LEN]);

impl IdentityId {
    pub const LEN: usize = 32;

    pub const fn from_bytes(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; Self::LEN] {
        &self.0
    }
}

/// Opaque device identifier authorized by an identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeviceId([u8; Self::LEN]);

impl DeviceId {
    pub const LEN: usize = 32;

    pub const fn from_bytes(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; Self::LEN] {
        &self.0
    }
}
