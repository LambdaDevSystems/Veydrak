//! Plugin engine: manifests, discovery and permission grants.
//!
//! Plugins are user-installed folders under `<config dir>/veydrak/plugins/<id>/`. Their UI runs in a
//! sandboxed iframe and can only reach host data through permissioned, host-mediated calls.
// Phase 1 lands the model and its tests before anything consumes it.
#![allow(dead_code)]

pub mod discovery;
pub mod manifest;
pub mod permissions;
