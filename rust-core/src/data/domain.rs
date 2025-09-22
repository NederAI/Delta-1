//! Core dataset definitions and contracts.
//!
//! TODO: Define schema parsing strategy that keeps zero-copy guarantees.
//! TODO: Introduce dataset lifecycle states (draft, active, deprecated) once retention policies are clear.

use crate::common::error::DeltaResult;

/// Opaque identifier for datasets.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DatasetId(String);

impl DatasetId {
    /// Construct a dataset identifier from a string slice.
    pub fn new<S: Into<String>>(value: S) -> Self {
        Self(value.into())
    }

    /// Borrow the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the identifier and return the owned string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

/// Simplistic representation of a dataset schema.
#[derive(Clone, Debug)]
pub struct Schema {
    pub definition_json: String,
    // TODO: Pre-compute parsed columns to speed up validation during ingest.
}

/// Dataset metadata stored alongside the raw data.
#[derive(Clone, Debug)]
pub struct Dataset {
    pub id: DatasetId,
    pub schema: Schema,
    pub created_ms: u128,
    pub rows: u64,
    // TODO: Track lineage information to connect datasets to upstream sources.
}

/// Repository contract for dataset persistence.
pub trait DataRepo {
    fn put_dataset(&self, dataset: &Dataset) -> DeltaResult<()>;
    fn get_dataset(&self, id: DatasetId) -> DeltaResult<Dataset>;
    // TODO: Add streaming read/write APIs to avoid loading entire datasets in memory.
}

impl Dataset {
    /// Convenience constructor used by scaffolding code.
    pub fn new(id: DatasetId, schema_json: String, created_ms: u128, rows: u64) -> Self {
        Self {
            id,
            schema: Schema {
                definition_json: schema_json,
            },
            created_ms,
            rows,
        }
    }
    // TODO: Add invariants to ensure schema and row count remain consistent.
}

// TODO: Provide lightweight validators that can be shared across services.
