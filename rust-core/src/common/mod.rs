//! Shared utilities that glue the different domains together.
//!
//! TODO: Decide whether the common module should expose a facade or
//!       require explicit imports per submodule for stronger boundaries.
pub mod buf;
pub mod config;
pub mod error;
pub mod ids;
pub mod json;
pub mod log;
pub mod time;

pub use error::{DeltaCode, DeltaError, DeltaResult};

// TODO: Re-export lightweight telemetry helpers when the logging format stabilises.
// TODO: Evaluate grouping time and logging concerns under a dedicated observability namespace.
