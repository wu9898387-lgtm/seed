#![forbid(unsafe_code)]

//! Seed core protocol primitives.
//!
//! This crate intentionally contains no social roles such as owner/admin/moderator.
//! It defines low-level identity, device authorization, spaces, signed events,
//! capabilities, plugin manifests, crypto boundaries, and event storage primitives.

pub mod capability;
pub mod crypto;
pub mod event;
pub mod id;
pub mod identity;
pub mod plugin;
pub mod space;
pub mod storage;

pub const PROTOCOL_VERSION: u16 = 1;
pub const PLUGIN_API_VERSION: u16 = 1;
