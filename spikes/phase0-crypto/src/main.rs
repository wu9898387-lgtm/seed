use seed_core::crypto::{verify_event_signature, EventSigningKey};

const MESSAGE: &[u8] = b"seed:phase0:event-signature:v1";

fn print_hex(label: &str, bytes: &[u8]) {
    print!("{label}=");
    for byte in bytes {
        print!("{byte:02x}");
    }
    println!();
}

fn main() {
    let secret = [0x11; 32];
    let signing_key = EventSigningKey::from_secret_bytes(&secret);
    let public_key = signing_key.verifying_key_bytes();
    let signature = signing_key.sign(MESSAGE);

    verify_event_signature(&public_key, MESSAGE, &signature)
        .expect("the deterministic Phase 0 vector must verify");

    println!("Seed Phase 0 Ed25519 spike");
    print_hex("public_key", &public_key);
    print_hex("signature", &signature);
}
