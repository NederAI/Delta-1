//! Domain types for model training and versioning.
//!
//! TODO: Encode semantic version identifiers with stronger typing.
//! TODO: Track parent dataset identifiers for lineage and reproducibility.

use crate::common::error::DeltaResult;
use crate::data::domain::DatasetId;

/// Identifier for a logical model family.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ModelId(pub u32);

impl ModelId {
    pub fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub fn raw(&self) -> u32 {
        self.0
    }
}

impl From<u32> for ModelId {
    fn from(value: u32) -> Self {
        ModelId(value)
    }
}

impl From<ModelId> for u32 {
    fn from(value: ModelId) -> Self {
        value.0
    }
}

/// Versioned model artefact metadata.
#[derive(Clone, Debug)]
pub struct ModelVersion {
    pub id: ModelId,
    pub version: String,
    pub artefact_path: String,
    // TODO: Add checksum/hash fields to detect corruption early.
}

/// Training configuration blob (mini JSON string for now).
#[derive(Clone, Debug)]
pub struct TrainConfig {
    pub raw: String,
    // TODO: Parse known hyperparameters eagerly for validation and ergonomics.
}

impl TrainConfig {
    pub fn new(raw: String) -> Self {
        Self { raw }
    }
}

/// Repository contract for model artefacts.
pub trait ModelRepo {
    fn put_model(&self, model: &ModelVersion) -> DeltaResult<()>;
    fn get_model(&self, id: ModelId) -> DeltaResult<ModelVersion>;
    // TODO: Introduce iterators over historical versions for rollback strategies.
}

/// Interface for components that can perform training.
pub trait Trainer {
    fn train(&self, dataset: DatasetId, cfg: &TrainConfig) -> DeltaResult<ModelVersion>;
    // TODO: Provide hooks for progress reporting and cancellation.
}
