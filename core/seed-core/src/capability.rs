use crate::id::{DeviceId, IdentityId, SpaceId};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    MemberInvite,
    MemberRemove,
    MessageSend,
    MessageDelete,
    SpaceCreateChild,
    PluginInstall,
    PluginRemove,
    PluginUpdate,
    PluginConfigure,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityRequest {
    pub actor: IdentityId,
    pub device: DeviceId,
    pub space: SpaceId,
    pub capability: Capability,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CapabilityDecision {
    Allow,
    Deny,
    /// The decision depends on an external governance workflow such as a vote.
    Pending {
        reason: String,
    },
}

pub trait CapabilityEvaluator {
    fn evaluate(&self, request: &CapabilityRequest) -> CapabilityDecision;
}

/// Safe baseline when no governance plugin claims a capability.
#[derive(Debug, Default)]
pub struct DefaultDeny;

impl CapabilityEvaluator for DefaultDeny {
    fn evaluate(&self, _request: &CapabilityRequest) -> CapabilityDecision {
        CapabilityDecision::Deny
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_implicit_creator_authority_exists() {
        let request = CapabilityRequest {
            actor: IdentityId::from_bytes([1; 32]),
            device: DeviceId::from_bytes([2; 32]),
            space: SpaceId::from_bytes([3; 32]),
            capability: Capability::MemberRemove,
        };

        assert_eq!(DefaultDeny.evaluate(&request), CapabilityDecision::Deny);
    }
}
