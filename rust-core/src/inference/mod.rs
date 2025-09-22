//! Inference domain providing synchronous and batch prediction APIs.
//!
//! TODO: Thread safe caches for model artefacts to reduce repeated disk reads.
//! TODO: Make worker pool configurable per endpoint once routing rules exist.

pub mod domain;
pub mod service;
pub mod workers;

pub use domain::Prediction;

// TODO: Export worker pool builders after benchmarking default sizes.
