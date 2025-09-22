//! Evaluation and drift detection scaffolding.
//!
//! TODO: Align terminology with external reporting pipeline (metrics.card, bias.card).
//! TODO: Determine retention window for evaluation artefacts.

pub mod domain;
pub mod service;

pub use domain::{DriftStats, EvalSuite};

// TODO: Provide streaming evaluators once online metrics are specified.
