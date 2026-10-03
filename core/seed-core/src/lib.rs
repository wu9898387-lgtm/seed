#![forbid(unsafe_code)]

//! Seed core protocol primitives.
//!
//! This crate intentionally contains no social roles such as owner/admin/moderator.
//! It defines low-level identity, device authorization, canonical Genesis records,
//! spaces, signed events, capabilities, plugin manifests, crypto boundaries, and
//! event storage primitives.

pub mod capability;
mod codec;
pub mod crypto;
pub mod event;
pub mod genesis;
pub mod id;
pub mod identity;
pub mod plugin;
pub mod space;
#[cfg(feature = "sqlite-storage")]
pub mod sqlite_storage;
pub mod storage;
pub mod transport;

pub const PROTOCOL_VERSION: u16 = 1;
pub const PLUGIN_API_VERSION: u16 = 1;
