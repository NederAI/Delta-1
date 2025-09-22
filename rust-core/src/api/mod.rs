//! Public entry points for foreign function interfaces.
//!
//! TODO: Add feature flags for alternative bindings (e.g. HTTP, gRPC) when needed.
//! TODO: Provide ABI version negotiation helpers.

pub mod ffi;

// TODO: Consider grouping low-level helpers for testing the FFI boundary.
