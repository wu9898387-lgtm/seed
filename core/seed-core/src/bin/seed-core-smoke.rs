use seed_core::{
    event::{Event, EventHeader},
    id::SpaceId,
    identity::{DeviceIdentity, RootIdentity},
    PROTOCOL_VERSION,
};

fn main() {
    let root = RootIdentity::generate().expect("root identity");
    let device = DeviceIdentity::generate().expect("device identity");
    let authorization = root.authorize_device(&device, 1, 0);

    let header = EventHeader {
        protocol_version: PROTOCOL_VERSION,
        space: SpaceId::from_bytes([0; 32]),
        author: root.document().id(),
        device: device.id(),
        sequence: 1,
        timestamp_ms: 0,
        schema: "seed.smoke/v1".to_owned(),
    };

    let event = Event::sign(
        root.document(),
        &authorization,
        &device,
        header,
        b"seed".to_vec(),
    )
    .expect("signed event");

    event
        .verify(root.document(), &authorization)
        .expect("verified event");

    println!(
        "seed-core protocol={} identity={} event={}",
        PROTOCOL_VERSION,
        root.document().id(),
        event.id()
    );
}
