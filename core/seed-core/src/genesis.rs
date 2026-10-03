use crate::identity::IdentityId;
use crate::plugin::{PluginId, PluginVersion};
use crate::space::{SpaceId, SpaceKind};

/// One plugin entry committed into Genesis.
///
/// Plugin configuration bytes must themselves be deterministic for that
/// plugin/version. Core treats them as opaque.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenesisPlugin<'a> {
    pub id: PluginId,
    pub version: PluginVersion,
    pub config: &'a [u8],
}

/// Fields covered by the Genesis creator signature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignableGenesis<'a> {
    pub space: SpaceId,
    pub kind: SpaceKind,
    pub creator: IdentityId,
    pub created_at_ms: u64,
    pub plugins: &'a [GenesisPlugin<'a>],
}
