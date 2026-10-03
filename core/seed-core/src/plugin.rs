use crate::{capability::Capability, id::PluginId, PLUGIN_API_VERSION};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PluginVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginPermission {
    EvaluateCapability(Capability),
    ReadEvents,
    ScopedStorage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginManifest {
    pub manifest_version: u16,
    pub plugin_api_version: u16,
    pub id: PluginId,
    pub version: PluginVersion,
    pub permissions: Vec<PluginPermission>,
}

impl PluginManifest {
    pub fn new(id: PluginId, version: PluginVersion) -> Self {
        Self {
            manifest_version: 1,
            plugin_api_version: PLUGIN_API_VERSION,
            id,
            version,
            permissions: Vec::new(),
        }
    }

    pub fn can_evaluate(&self, capability: &Capability) -> bool {
        self.permissions.iter().any(|permission| {
            matches!(permission, PluginPermission::EvaluateCapability(value) if value == capability)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undeclared_capability_hook_is_denied() {
        let manifest = PluginManifest::new(
            PluginId::from_bytes([7; 32]),
            PluginVersion {
                major: 0,
                minor: 1,
                patch: 0,
            },
        );

        assert!(!manifest.can_evaluate(&Capability::MemberRemove));
    }
}
