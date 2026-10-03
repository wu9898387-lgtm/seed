use crate::id::{GenesisId, SpaceId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SpaceKind {
    Direct = 1,
    Group = 2,
    Tree = 3,
    Branch = 4,
    Channel = 5,
}

impl SpaceKind {
    pub(crate) const fn wire_value(self) -> u8 {
        self as u8
    }

    pub(crate) const fn from_wire(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Direct),
            2 => Some(Self::Group),
            3 => Some(Self::Tree),
            4 => Some(Self::Branch),
            5 => Some(Self::Channel),
            _ => None,
        }
    }
}

/// Protocol-level description of a space.
///
/// Deliberately contains no owner/admin/moderator fields. Social authority is
/// expected to be derived from plugin state and capability evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpaceDescriptor {
    pub id: SpaceId,
    pub kind: SpaceKind,
    pub genesis: Option<GenesisId>,
}
