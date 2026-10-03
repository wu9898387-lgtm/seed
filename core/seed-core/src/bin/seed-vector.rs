use seed_core::{
    event::{Event, EventHeader},
    id::SpaceId,
    identity::{DeviceIdentity, RootIdentity},
    PROTOCOL_VERSION,
};

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use core::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("hex write");
    }
    out
}

fn main() {
    let root = RootIdentity::from_secret_bytes([1u8; 32]);
    let device = DeviceIdentity::from_secret_bytes([2u8; 32]);
    let authorization = root.authorize_device(&device, 1, 1_700_000_000_000);

    let header = EventHeader {
        protocol_version: PROTOCOL_VERSION,
        space: SpaceId::from_bytes([3u8; 32]),
        author: root.document().id(),
        device: device.id(),
        sequence: 7,
        timestamp_ms: 1_700_000_000_123,
        schema: "seed.message.text/v1".to_owned(),
    };

    let event = Event::sign(
        root.document(),
        &authorization,
        &device,
        header,
        b"hello".to_vec(),
    )
    .expect("event");

    println!(
        "root_public={}",
        hex(root.document().root_public_key().as_bytes())
    );
    println!("identity_id={}", root.document().id());
    println!("device_public={}", hex(device.public_key().as_bytes()));
    println!("device_id={}", device.id());
    println!(
        "device_authorization_signature={}",
        hex(authorization.root_signature().as_bytes())
    );
    println!("event_id={}", event.id());
    println!("event_signature={}", hex(event.signature().as_bytes()));
}
