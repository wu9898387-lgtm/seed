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

#[test]
fn identity_device_event_vector_v1() {
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
    .unwrap();

    assert_eq!(
        hex(root.document().root_public_key().as_bytes()),
        "8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c"
    );
    assert_eq!(
        root.document().id().to_string(),
        "00524173a9a0f74fe50a3ffde32c2c8cf013c734a39ae7847fb56fd98ddefdb9"
    );
    assert_eq!(
        hex(device.public_key().as_bytes()),
        "8139770ea87d175f56a35466c34c7ecccb8d8a91b4ee37a25df60f5b8fc9b394"
    );
    assert_eq!(
        device.id().to_string(),
        "73c192ef5ee4e3f2e1815f48f09cf8f3d8d958d6e347aa07b456389e03554c4a"
    );
    assert_eq!(
        hex(authorization.root_signature().as_bytes()),
        "5c762762b103c4fbbd382a08f900e5d4e2ffa0a4be1c7d0f6bce65ce1a1d8b4b5f80b9d7d4ac736afaba7a546263aeab13d4ec84814579a63ff3d260e6f7570f"
    );
    assert_eq!(
        event.id().to_string(),
        "c620f326a53b852d0a0b5a1b128b24da0a4c750a9473a135d5ef6467b5e71ff5"
    );
    assert_eq!(
        hex(event.signature().as_bytes()),
        "f68705f312deb509321dc190f7a8fa8840c60547d8b8b7284b073ed3be1bb3356a3589f50902de1705256caf253c2bfc0a643fb5d8a0f22e06ffb97c2087ed0c"
    );

    event.verify(root.document(), &authorization).unwrap();
}
