#![no_std]
#![forbid(unsafe_code)]

pub mod capability;
#[cfg(feature = "crypto-ed25519")]
pub mod crypto;
pub mod event;
pub mod identity;
pub mod plugin;
pub mod space;
pub mod transport;

/// Protocol version for the Phase 0 spike.
///
/// This is deliberately pre-stable. It is not a compatibility promise.
pub const PROTOCOL_MAJOR: u16 = 0;
pub const PROTOCOL_MINOR: u16 = 1;
