use seed_core::codec::encode_genesis_for_signing;
use seed_core::crypto::{verify_event_signature, EventSigningKey};
use seed_core::genesis::{GenesisPlugin, SignableGenesis};
use seed_core::identity::IdentityId;
use seed_core::plugin::{PluginId, PluginVersion};
use seed_core::space::{SpaceId, SpaceKind};

fn main() {
    let plugins = [
        GenesisPlugin {
            id: PluginId::from_bytes([0x10; 16]),
            version: PluginVersion {
                major: 1,
                minor: 0,
                patch: 0,
            },
            config: b"owner=alice",
        },
        GenesisPlugin {
            id: PluginId::from_bytes([0x20; 16]),
            version: PluginVersion {
                major: 1,
                minor: 2,
                patch: 3,
            },
            config: b"",
        },
    ];
    let genesis = SignableGenesis {
        space: SpaceId::from_bytes([0x55; 32]),
        kind: SpaceKind::GROUP,
        creator: IdentityId::from_bytes([0x33; 32]),
        created_at_ms: 1_700_000_000_000,
        plugins: &plugins,
    };

    let mut canonical = [0u8; 151];
    let written = encode_genesis_for_signing(&genesis, &mut canonical)
        .expect("fixed Phase 0 Genesis must encode");
    let signing_key = EventSigningKey::from_secret_bytes(&[0x11; 32]);
    let public_key = signing_key.verifying_key_bytes();
    let signature = signing_key.sign(&canonical[..written]);

    verify_event_signature(&public_key, &canonical[..written], &signature)
        .expect("Genesis signature must verify");

    println!("Seed Phase 0 Genesis spike");
    println!("canonical_bytes={written}");
    println!("plugins={}", plugins.len());
}
