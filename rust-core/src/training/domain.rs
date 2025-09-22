//! Domain types for model training and versioning.
//!
//! TODO: Encode semantic version identifiers with stronger typing.
//! TODO: Track parent dataset identifiers for lineage and reproducibility.

use crate::common::error::DeltaResult;
use crate::common::json;
use crate::data::domain::DatasetId;

/// Identifier for a logical model family.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ModelId(String);

impl ModelId {
    /// Construct a model identifier from a string slice.
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

/// Version label wrapper to avoid mixing with model identifiers.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct VersionName(String);

impl VersionName {
    pub fn new<S: Into<String>>(value: S) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Supported model kinds defined by the product roadmap.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ModelKind {
    TabularLogistic,
    TabularGradientBoosting,
    TextMiniLm,
}

impl Default for ModelKind {
    fn default() -> Self {
        ModelKind::TabularLogistic
    }
}

/// Metadata associated with a model version that affects routing and governance.
#[derive(Clone, Debug, Default)]
pub struct ModelMetadata {
    pub dp: DifferentialPrivacy,
    pub fairness: Option<FairnessReport>,
}

/// Differential privacy configuration snapshot.
#[derive(Clone, Debug, Default)]
pub struct DifferentialPrivacy {
    pub enabled: bool,
    pub epsilon: f32,
    pub delta: f32,
    pub clip: f32,
    pub noise_multiplier: f32,
}

/// Simplified fairness metrics captured during evaluation.
#[derive(Clone, Debug, Default)]
pub struct FairnessReport {
    pub delta_tpr: f32,
    pub delta_fpr: f32,
    pub delta_ppv: f32,
}

/// Versioned model artefact metadata.
#[derive(Clone, Debug)]
pub struct ModelVersion {
    pub id: ModelId,
    pub version: VersionName,
    pub kind: ModelKind,
    pub artefact_path: String,
    pub metadata: ModelMetadata,
    // TODO: Add checksum/hash fields to detect corruption early.
}

/// Training configuration blob (mini JSON string parsed into a structured spec).
#[derive(Clone, Debug)]
pub struct TrainConfig {
    pub raw: String,
    pub spec: TrainSpec,
}

impl TrainConfig {
    pub fn parse(raw: String) -> DeltaResult<Self> {
        Ok(Self {
            spec: TrainSpec::from_raw(&raw),
            raw,
        })
    }

    pub fn model_kind(&self) -> ModelKind {
        self.spec.model_kind
    }

    pub fn fairness(&self) -> Option<&FairnessReport> {
        self.spec.fairness.as_ref()
    }

    pub fn dp(&self) -> &DifferentialPrivacy {
        &self.spec.dp
    }
}

/// Internal training specification derived from JSON.
#[derive(Clone, Debug, Default)]
pub struct TrainSpec {
    pub model_kind: ModelKind,
    pub dp: DifferentialPrivacy,
    pub fairness: Option<FairnessReport>,
}

impl TrainSpec {
    fn from_raw(raw: &str) -> Self {
        let model_kind = match json::extract_string(raw, "model_kind").as_deref() {
            Some("tabular_gbdt") => ModelKind::TabularGradientBoosting,
            Some("text_minilm") => ModelKind::TextMiniLm,
            _ => ModelKind::TabularLogistic,
        };

        let dp_section = json::extract_object(raw, "dp").unwrap_or("{}");
        let dp = DifferentialPrivacy {
            enabled: json::extract_bool(dp_section, "enabled").unwrap_or(false),
            epsilon: json::extract_number(dp_section, "epsilon").unwrap_or(3.0),
            delta: json::extract_number(dp_section, "delta").unwrap_or(1e-5),
            clip: json::extract_number(dp_section, "clip").unwrap_or(1.0),
            noise_multiplier: json::extract_number(dp_section, "noise_multiplier").unwrap_or(1.0),
        };

        let fairness = json::extract_object(raw, "fairness").map(|section| FairnessReport {
            delta_tpr: json::extract_number(section, "delta_tpr").unwrap_or_default(),
            delta_fpr: json::extract_number(section, "delta_fpr").unwrap_or_default(),
            delta_ppv: json::extract_number(section, "delta_ppv").unwrap_or_default(),
        });

        Self {
            model_kind,
            dp,
            fairness,
        }
    }
}

/// Repository contract for model artefacts.
pub trait ModelRepo {
    fn put_model(&self, model: &ModelVersion) -> DeltaResult<()>;
    fn get_model(&self, id: &ModelId, version: &VersionName) -> DeltaResult<ModelVersion>;
    // TODO: Introduce iterators over historical versions for rollback strategies.
}

/// Interface for components that can perform training.
pub trait Trainer {
    fn train(&self, dataset: DatasetId, cfg: &TrainConfig) -> DeltaResult<ModelVersion>;
    // TODO: Provide hooks for progress reporting and cancellation.
}
