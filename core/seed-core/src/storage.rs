use std::collections::HashSet;

use crate::{event::Event, id::SpaceId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppendOutcome {
    Inserted,
    Duplicate,
}

pub trait EventStore {
    fn append(&mut self, event: Event) -> AppendOutcome;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn events_for_space(&self, space: &SpaceId) -> Vec<&Event>;
}

#[derive(Debug, Default)]
pub struct InMemoryEventStore {
    seen: HashSet<crate::id::EventId>,
    events: Vec<Event>,
}

impl EventStore for InMemoryEventStore {
    fn append(&mut self, event: Event) -> AppendOutcome {
        if !self.seen.insert(event.header.id) {
            return AppendOutcome::Duplicate;
        }
        self.events.push(event);
        AppendOutcome::Inserted
    }

    fn len(&self) -> usize {
        self.events.len()
    }

    fn events_for_space(&self, space: &SpaceId) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|event| &event.header.space == space)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        crypto::Signature,
        event::{Event, EventHeader},
        id::{DeviceId, EventId, IdentityId, SpaceId},
    };

    use super::*;

    fn event(id: u8, space: u8) -> Event {
        Event {
            header: EventHeader {
                protocol_version: 1,
                id: EventId::from_bytes([id; 32]),
                space: SpaceId::from_bytes([space; 32]),
                author: IdentityId::from_bytes([3; 32]),
                device: DeviceId::from_bytes([4; 32]),
                sequence: id as u64,
                timestamp_ms: 0,
                schema: "test/v1".to_owned(),
            },
            payload: vec![],
            signature: Signature::from_bytes(vec![]),
        }
    }

    #[test]
    fn duplicate_event_is_idempotent() {
        let mut store = InMemoryEventStore::default();
        assert_eq!(store.append(event(1, 9)), AppendOutcome::Inserted);
        assert_eq!(store.append(event(1, 9)), AppendOutcome::Duplicate);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn query_is_scoped_to_space() {
        let mut store = InMemoryEventStore::default();
        store.append(event(1, 9));
        store.append(event(2, 8));

        assert_eq!(store.events_for_space(&SpaceId::from_bytes([9; 32])).len(), 1);
    }
}
