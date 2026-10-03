use crate::identity::IdentityId;
use crate::space::SpaceId;

/// A stable capability name exposed by Core.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CapabilityId(&'static str);

impl CapabilityId {
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// Capability names required by the first MVP slices.
pub mod builtin {
    use super::CapabilityId;

    pub const MEMBER_INVITE: CapabilityId = CapabilityId::new("member.invite");
    pub const MEMBER_REMOVE: CapabilityId = CapabilityId::new("member.remove");
    pub const MESSAGE_SEND: CapabilityId = CapabilityId::new("message.send");
    pub const MESSAGE_DELETE: CapabilityId = CapabilityId::new("message.delete");
    pub const SPACE_CREATE_CHILD: CapabilityId = CapabilityId::new("space.create_child");
    pub const PLUGIN_INSTALL: CapabilityId = CapabilityId::new("plugin.install");
    pub const PLUGIN_REMOVE: CapabilityId = CapabilityId::new("plugin.remove");
    pub const PLUGIN_UPDATE: CapabilityId = CapabilityId::new("plugin.update");
    pub const PLUGIN_CONFIGURE: CapabilityId = CapabilityId::new("plugin.configure");
}

/// Result of governance evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    Allow,
    Deny,
    Pending { ticket: u64 },
}

/// A request to invoke a sensitive Core capability.
///
/// arguments is opaque at this layer so governance can be evaluated without
/// embedding social roles into the Core type system.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilityRequest<'a> {
    pub capability: CapabilityId,
    pub actor: IdentityId,
    pub space: SpaceId,
    pub arguments: &'a [u8],
}

/// Governance plugins/runtime adapters implement this boundary.
///
/// Core has no creator/owner bypass: the caller must receive a decision from
/// the active policy path.
pub trait CapabilityPolicy {
    fn evaluate(&self, request: &CapabilityRequest<'_>) -> Decision;
}

#[cfg(test)]
mod tests {
    use super::{builtin, CapabilityPolicy, CapabilityRequest, Decision};
    use crate::identity::IdentityId;
    use crate::space::SpaceId;

    struct DenyAll;

    impl CapabilityPolicy for DenyAll {
        fn evaluate(&self, _request: &CapabilityRequest<'_>) -> Decision {
            Decision::Deny
        }
    }

    #[test]
    fn capability_requires_policy_decision() {
        let request = CapabilityRequest {
            capability: builtin::MEMBER_REMOVE,
            actor: IdentityId::from_bytes([1; 32]),
            space: SpaceId::from_bytes([2; 32]),
            arguments: &[],
        };

        assert_eq!(DenyAll.evaluate(&request), Decision::Deny);
    }
}
