//! Domain definitions for inference requests and responses.
//!
//! TODO: Model structured output beyond raw JSON once payload schema is finalised.
//! TODO: Track latency percentiles and drift metrics in a central structure.

use crate::common::error::DeltaResult;
use crate::training::domain::ModelVersion;

/// Result of a single inference call.
#[derive(Clone, Debug)]
pub struct Prediction {
    pub json: String,
    pub latency_ms: u32,
    pub confidence: f32,
    // TODO: Add rich metadata fields for routing and auditing.
}

/// Engine abstraction to decouple service orchestration from concrete implementations.
pub trait InferEngine {
    fn infer(&self, model: &ModelVersion, input_json: &str) -> DeltaResult<Prediction>;
    // TODO: Add batch inference contract and streaming variants.
}
