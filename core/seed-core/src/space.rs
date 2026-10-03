/// Stable identifier for a communication space.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SpaceId([u8; 32]);

impl SpaceId {
    pub const LEN: usize = 32;

    pub const fn from_bytes(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; Self::LEN] {
        &self.0
    }
}

/// Protocol-level space category.
///
/// Social roles such as owner/admin/moderator intentionally do not appear
/// here. They belong to governance plugins.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SpaceKind(u16);

impl SpaceKind {
    pub const DIRECT: Self = Self(1);
    pub const GROUP: Self = Self(2);
    pub const TREE: Self = Self(3);
    pub const BRANCH: Self = Self(4);
    pub const CHANNEL: Self = Self(5);

    pub const fn from_code(code: u16) -> Self {
        Self(code)
    }

    pub const fn code(self) -> u16 {
        self.0
    }
}
