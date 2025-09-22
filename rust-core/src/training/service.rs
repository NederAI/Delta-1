//! Service layer orchestrating dataset ingestion and model training.
//!
//! TODO: Implement deterministic training outputs to ensure reproducibility across runs.
//! TODO: Record artefact manifests and metrics for downstream evaluation pipelines.

use crate::common::error::{DeltaError, DeltaResult};
use crate::common::ids::SimpleHash;
use crate::data::domain::DatasetId;

use super::domain::{ModelId, ModelVersion, TrainConfig};

/// Train a model for the given dataset.
pub fn train(dataset: DatasetId, cfg_json: &str) -> DeltaResult<ModelId> {
    let _cfg = TrainConfig::new(cfg_json.to_string());
    // TODO: Load dataset metadata and construct deterministic training artefacts.
    // TODO: Store the produced artefact via the filesystem repository.

    let mut hasher = SimpleHash::new();
    hasher.update(&dataset.raw().to_le_bytes());
    hasher.update(cfg_json.as_bytes());
    let model_id = ModelId::new(hasher.finish32());

    Ok(model_id)
}

/// Load the current model version for inference.
pub fn load_model(_id: ModelId) -> DeltaResult<ModelVersion> {
    // TODO: Fetch metadata from the repository once persisted by `train`.
    Err(DeltaError::not_implemented("training::service::load_model"))
}

// TODO: Provide APIs for listing versions and promoting candidates to production.
