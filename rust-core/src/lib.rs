// lib.rs - centrale orchestrator
pub mod common;
pub mod data;
pub mod training;
pub mod inference;
pub mod evaluation;
pub mod api;

pub use api::ffi::{/* data_ingest, train_model, run_inference */};
