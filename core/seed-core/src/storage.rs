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
        if !self.seen.insert(event.id()) {
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
            .filter(|event| &event.header().space == space)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        event::{Event, EventHeader},
        id::SpaceId,
        identity::{DeviceIdentity, RootIdentity},
        PROTOCOL_VERSION,
    };

    use super::*;

    fn event(sequence: u64, space: u8) -> Event {
        let root = RootIdentity::generate().unwrap();
        let device = DeviceIdentity::generate().unwrap();
        let authorization = root.authorize_device(&device, 1, 0);
        let header = EventHeader {
            protocol_version: PROTOCOL_VERSION,
            space: SpaceId::from_bytes([space; 32]),
            author: root.document().id(),
            device: device.id(),
            sequence,
            timestamp_ms: 0,
            schema: "test/v1".to_owned(),
        };

        Event::sign(
            root.document(),
            &authorization,
            &device,
            header,
            vec![sequence as u8],
        )
        .unwrap()
    }

    #[test]
    fn duplicate_event_is_idempotent() {
        let event = event(1, 9);
        let duplicate = event.clone();
        let mut store = InMemoryEventStore::default();

        assert_eq!(store.append(event), AppendOutcome::Inserted);
        assert_eq!(store.append(duplicate), AppendOutcome::Duplicate);
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
