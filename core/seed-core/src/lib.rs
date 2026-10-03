#![forbid(unsafe_code)]

//! Seed core protocol primitives.
//!
//! This crate intentionally contains no social roles such as owner/admin/moderator.
//! It defines only low-level identities, spaces, events, capabilities, plugin
//! manifests, crypto interfaces, and event storage primitives.

pub mod capability;
pub mod crypto;
pub mod event;
pub mod id;
pub mod plugin;
pub mod space;
pub mod storage;

pub const PROTOCOL_VERSION: u16 = 1;
pub const PLUGIN_API_VERSION: u16 = 1;
