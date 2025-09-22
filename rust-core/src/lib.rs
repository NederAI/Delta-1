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

pub use data::service::{export_datasheet, ingest_file as core_data_ingest};
pub use inference::service::{infer_with_ctx as core_infer_with_ctx, register_active_model};
pub use training::service::{
    export_model_card, load_model as core_load_model, train as core_train,
};

// TODO: Re-export evaluation entry points when the reporting format settles.
// TODO: Consider providing a top-level builder to assemble repositories with shared config.
