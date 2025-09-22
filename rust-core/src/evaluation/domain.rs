//! Domain primitives for evaluation and drift tracking.
//!
//! TODO: Define richer metric structures (AUC/F1 etc.) with deterministic serialisation.
//! TODO: Incorporate fairness and bias auditing requirements.

use crate::common::error::DeltaResult;
use crate::training::domain::ModelVersion;

/// Summary of evaluation metrics for a particular model.
#[derive(Clone, Debug)]
pub struct EvalSuite {
    pub model: ModelVersion,
    pub metrics_card: String,
    // TODO: Store computed statistics in a structured format once schema is final.
}

/// Drift statistics placeholder.
#[derive(Clone, Debug, Default)]
pub struct DriftStats {
    pub psi: f32,
    pub ks: f32,
    // TODO: Track histograms and time windows for more granular alerts.
}

/// Repository contract placeholder for evaluation artefacts.
pub trait EvalRepo {
    fn put_suite(&self, suite: &EvalSuite) -> DeltaResult<()>;
    fn get_latest(&self, model: &ModelVersion) -> DeltaResult<EvalSuite>;
    // TODO: Provide diff utilities comparing historical evaluation reports.
}
