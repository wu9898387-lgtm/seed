use crate::identity::{DeviceId, IdentityId};
use crate::space::SpaceId;

/// Content-addressed event identifier.
///
/// Phase 0 treats the bytes as opaque until canonical encoding and hashing are
/// selected by ADR.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EventId([u8; Self::LEN]);

impl EventId {
    pub const LEN: usize = 32;

    pub const fn from_bytes(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; Self::LEN] {
        &self.0
    }
}

/// Extensible event type code.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EventKind(u16);

impl EventKind {
    pub const MESSAGE: Self = Self(1);
    pub const GOVERNANCE: Self = Self(2);
    pub const PLUGIN: Self = Self(3);

    pub const fn from_code(code: u16) -> Self {
        Self(code)
    }

    pub const fn code(self) -> u16 {
        self.0
    }
}

/// Minimal signed-event boundary.
///
/// Serialization, hashing, signature algorithm, clocks, and causal ordering
/// are intentionally not frozen in this spike.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventEnvelope<'a> {
    pub id: EventId,
    pub space: SpaceId,
    pub author: IdentityId,
    pub device: DeviceId,
    pub kind: EventKind,
    pub payload: &'a [u8],
    pub signature: &'a [u8],
}
