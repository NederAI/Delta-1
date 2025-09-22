//! Data domain: ingest, validation and persistence of datasets.
//!
//! TODO: Split filesystem persistence into feature-gated modules when alternative backends appear.
//! TODO: Define clear ownership boundaries for dataset lifecycle events.

pub mod domain;
pub mod repo_fs;
pub mod service;

pub use domain::{Dataset, DatasetId, Schema};

// TODO: Re-export specialised ingest reports once the format is defined.
