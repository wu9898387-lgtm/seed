use seed_core::{
    id::SpaceId,
    space::{SpaceDescriptor, SpaceKind},
    PROTOCOL_VERSION,
};

fn main() {
    let space = SpaceDescriptor {
        id: SpaceId::from_bytes([0; 32]),
        kind: SpaceKind::Group,
        genesis_event: None,
    };

    println!(
        "seed-core protocol={} smoke={:?}",
        PROTOCOL_VERSION, space.kind
    );
}
