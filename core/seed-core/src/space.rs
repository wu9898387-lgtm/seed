use crate::id::{EventId, SpaceId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpaceKind {
    Direct,
    Group,
    Tree,
    Branch,
    Channel,
}

/// Protocol-level description of a space.
///
/// Deliberately contains no owner/admin/moderator fields. Social authority is
/// expected to be derived from plugin state and capability evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpaceDescriptor {
    pub id: SpaceId,
    pub kind: SpaceKind,
    pub genesis_event: Option<EventId>,
}
