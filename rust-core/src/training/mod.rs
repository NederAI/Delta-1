//! Training domain responsible for model lifecycle management.
//!
//! TODO: Separate deterministic mock training from pluggable engines.
//! TODO: Add audit logging for every artefact write once requirements are clear.

pub mod domain;
pub mod repo_fs;
pub mod service;

pub use domain::{ModelId, ModelVersion, TrainConfig};

// TODO: Re-export trainer traits when multiple engines are available.
