//! Delta 1 core library - orchestrates domain modules.
//!
//! TODO: Wire module initialisation (config, repositories) once bootstrap sequence is defined.
//! TODO: Document the stability guarantees provided by the crate-level exports.

pub mod api;
pub mod common;
pub mod data;
pub mod evaluation;
pub mod inference;
pub mod training;

pub use data::service::ingest_file as core_data_ingest;
pub use inference::service::infer as core_infer;
pub use training::service::{load_model, train};

// TODO: Re-export evaluation entry points when the reporting format settles.
// TODO: Consider providing a top-level builder to assemble repositories with shared config.
