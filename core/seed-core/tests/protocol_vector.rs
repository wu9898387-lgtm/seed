use seed_core::{
    event::{Event, EventHeader},
    genesis::{GenesisDraft, GenesisPlugin, GenesisRecord},
    id::{PluginDigest, PluginId},
    identity::{DeviceIdentity, RootIdentity},
    plugin::PluginVersion,
    space::SpaceKind,
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
fn identity_genesis_event_vector_v1() {
    let root = RootIdentity::from_secret_bytes([1u8; 32]);
    let device = DeviceIdentity::from_secret_bytes([2u8; 32]);
    let authorization = root.authorize_device(&device, 1, 1_700_000_000_000);

    let mut draft = GenesisDraft::new(
        SpaceKind::Group,
        root.document(),
        &device,
        1_700_000_000_100,
    );
    draft
        .add_plugin(
            GenesisPlugin::new(
                PluginId::from_bytes([5u8; 32]),
                PluginVersion {
                    major: 1,
                    minor: 2,
                    patch: 3,
                },
                PluginDigest::from_bytes([6u8; 32]),
                b"owner=creator".to_vec(),
            )
            .unwrap(),
        )
        .unwrap();

    let genesis = draft
        .activate(root.document(), &authorization, &device)
        .unwrap();
    let genesis_bytes = genesis.canonical_bytes().unwrap();
    let decoded_genesis = GenesisRecord::from_canonical_bytes(&genesis_bytes).unwrap();

    let header = EventHeader {
        protocol_version: PROTOCOL_VERSION,
        space: genesis.space_id(),
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
        genesis.id().to_string(),
        "27f4890bfbd715f8f49052baeaf99835b3f742c5244b40ba53ef5722c89bc823"
    );
    assert_eq!(
        genesis.space_id().to_string(),
        "37d08795ca6c64054be999fd42e7d8fc797a097db1ff84bf5c06bb08fec6ebe8"
    );
    assert_eq!(
        hex(genesis.signature().as_bytes()),
        "50ec0caea3c14516945a8870b631e22caa3a4406efdd21b204a74415e289cb50afa4b9b3bf4a552d0c65697e82c39f78d9394065e5afda113e4f45fa0f39e308"
    );
    assert_eq!(
        hex(&genesis_bytes),
        "5347454e0001000000a6000100010200524173a9a0f74fe50a3ffde32c2c8cf013c734a39ae7847fb56fd98ddefdb973c192ef5ee4e3f2e1815f48f09cf8f3d8d958d6e347aa07b456389e03554c4a0000018bcfe568640001050505050505050505050505050505050505050505050505050505050505050500010002000306060606060606060606060606060606060606060606060606060606060606060000000d6f776e65723d63726561746f7250ec0caea3c14516945a8870b631e22caa3a4406efdd21b204a74415e289cb50afa4b9b3bf4a552d0c65697e82c39f78d9394065e5afda113e4f45fa0f39e308"
    );
    assert_eq!(
        event.id().to_string(),
        "d20549bd5b6e2bae07ea7ea15fd76150e67c4e20bb3699a46eb37f66245134bc"
    );
    assert_eq!(
        hex(event.signature().as_bytes()),
        "65f8d1f5be10a11a16acedfa3251f903b37d910483a1d4c1bb221687eeba8bae89e11a025a285e1be2e726f5281ae25cf2ebb266824b5b98177848c51ec9950e"
    );

    decoded_genesis
        .verify(root.document(), &authorization)
        .unwrap();
    event.verify(root.document(), &authorization).unwrap();
}
