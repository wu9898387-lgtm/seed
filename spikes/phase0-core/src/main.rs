use seed_core::capability::{
    builtin, CapabilityPolicy, CapabilityRequest, Decision,
};
use seed_core::event::EventEnvelope;
use seed_core::identity::IdentityId;
use seed_core::space::SpaceId;
use seed_core::{PROTOCOL_MAJOR, PROTOCOL_MINOR};

struct DenyAll;

impl CapabilityPolicy for DenyAll {
    fn evaluate(&self, _request: &CapabilityRequest<'_>) -> Decision {
        Decision::Deny
    }
}

fn main() {
    let actor = IdentityId::from_bytes([1; 32]);
    let space = SpaceId::from_bytes([2; 32]);
    let request = CapabilityRequest {
        capability: builtin::MEMBER_REMOVE,
        actor,
        space,
        arguments: &[],
    };

    println!("Seed Phase 0 core kernel spike");
    println!("protocol={PROTOCOL_MAJOR}.{PROTOCOL_MINOR}");
    println!(
        "capability={} decision={:?}",
        request.capability.as_str(),
        DenyAll.evaluate(&request)
    );
    println!(
        "event_envelope_size={} bytes",
        core::mem::size_of::<EventEnvelope<'static>>()
    );
}
