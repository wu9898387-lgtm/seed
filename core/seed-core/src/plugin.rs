use crate::capability::CapabilityId;

/// Opaque plugin identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PluginId([u8; Self::LEN]);

impl PluginId {
    pub const LEN: usize = 16;

    pub const fn from_bytes(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; Self::LEN] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PluginVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

/// Minimal manifest surface for the runtime spike.
///
/// Filesystem, network, UI, storage, and secret permissions are intentionally
/// deferred until the sandbox/runtime ADR is tested.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PluginManifest<'a> {
    pub id: PluginId,
    pub version: PluginVersion,
    pub requested_capabilities: &'a [CapabilityId],
}
