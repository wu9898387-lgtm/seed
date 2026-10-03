use seed_core::codec::{encode_event_for_signing, SignableEvent};
use seed_core::crypto::{verify_event_signature, EventSigningKey};
use seed_core::event::EventKind;
use seed_core::identity::{DeviceId, IdentityId};
use seed_core::space::SpaceId;

fn main() {
    let event = SignableEvent {
        space: SpaceId::from_bytes([0x22; 32]),
        author: IdentityId::from_bytes([0x33; 32]),
        device: DeviceId::from_bytes([0x44; 32]),
        kind: EventKind::MESSAGE,
        payload: b"hello",
    };

    let mut canonical = [0u8; 119];
    let written = encode_event_for_signing(&event, &mut canonical)
        .expect("the fixed Phase 0 Event vector must encode");

    let signing_key = EventSigningKey::from_secret_bytes(&[0x11; 32]);
    let public_key = signing_key.verifying_key_bytes();
    let signature = signing_key.sign(&canonical[..written]);

    verify_event_signature(&public_key, &canonical[..written], &signature)
        .expect("the canonical Event signature must verify");

    println!("Seed Phase 0 canonical Event spike");
    println!("canonical_bytes={written}");
    println!("signature_bytes={}", signature.len());
}
