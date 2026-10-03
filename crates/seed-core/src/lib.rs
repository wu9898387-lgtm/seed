//! Seed experimental core kernel.
//!
//! This crate is intentionally tiny during Phase 0. Social roles such as
//! owner/admin/moderator must not become Core primitives.

/// Protocol generation used by the current experimental test vectors.
pub const PROTOCOL_VERSION: u16 = 0;

/// Structural space kinds known by Core.
///
/// These values describe structure, not governance or social authority.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceKind {
    Direct = 0,
    Group = 1,
    Tree = 2,
    Branch = 3,
    Channel = 4,
}

/// Result of evaluating a capability request.
///
/// Pending is intentionally generic: voting, multisig, approval and
/// timelocks belong to plugins rather than Core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authorization {
    Allow,
    Deny,
    Pending([u8; 32]),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_kind_wire_values_are_stable_for_v0() {
        assert_eq!(SpaceKind::Direct as u8, 0);
        assert_eq!(SpaceKind::Group as u8, 1);
        assert_eq!(SpaceKind::Tree as u8, 2);
        assert_eq!(SpaceKind::Branch as u8, 3);
        assert_eq!(SpaceKind::Channel as u8, 4);
    }

    #[test]
    fn protocol_starts_experimental() {
        assert_eq!(PROTOCOL_VERSION, 0);
    }
}
