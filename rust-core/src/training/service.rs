//! Service layer orchestrating dataset ingestion and model training.
//!
//! Enforces the model design guardrails defined in the product brief: fixed
//! model families, fairness gates and differential privacy bounds. Metadata is
//! captured in-memory for now so the PHP layer can export model cards without
//! a persistent store.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::common::error::{DeltaError, DeltaResult};
use crate::common::ids::SimpleHash;
use crate::common::time;
use crate::data::domain::DatasetId;

use super::domain::{ModelId, ModelKind, ModelMetadata, ModelVersion, TrainConfig, VersionName};

const MAX_EPSILON: f32 = 3.0;
const MAX_DELTA: f32 = 1e-5;
const MAX_DELTA_TPR: f32 = 0.05;
const MAX_DELTA_FPR: f32 = 0.03;
const MAX_DELTA_PPV: f32 = 0.04;

#[derive(Default)]
struct ModelRegistry {
    entries: HashMap<(String, String), ModelVersion>,
    latest: HashMap<String, String>,
}

impl ModelRegistry {
    fn insert(&mut self, model: ModelVersion) {
        let key = (
            model.id.as_str().to_string(),
            model.version.as_str().to_string(),
        );
        self.latest.insert(
            model.id.as_str().to_string(),
            model.version.as_str().to_string(),
        );
        self.entries.insert(key, model);
    }

    fn get(&self, id: &ModelId, version: &VersionName) -> Option<ModelVersion> {
        let key = (id.as_str().to_string(), version.as_str().to_string());
        self.entries.get(&key).cloned()
    }

    fn latest(&self, id: &ModelId) -> Option<ModelVersion> {
        let version = self.latest.get(id.as_str())?;
        let key = (id.as_str().to_string(), version.clone());
        self.entries.get(&key).cloned()
    }
}

fn registry() -> &'static Mutex<ModelRegistry> {
    static REGISTRY: OnceLock<Mutex<ModelRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(ModelRegistry::default()))
}

/// Train a model for the given dataset.
pub fn train(dataset: DatasetId, cfg_json: &str) -> DeltaResult<ModelVersion> {
    let cfg = TrainConfig::parse(cfg_json.to_string())?;
    enforce_dp(&cfg)?;
    enforce_fairness(&cfg)?;

    let model_id = make_model_id(&dataset, cfg_json, cfg.model_kind());
    let version = VersionName::new(format!("v{}", time::now_ms()));
    let artefact_path = format!(
        "models/{}/{}/model.bin",
        model_id.as_str(),
        version.as_str()
    );

    let model = ModelVersion {
        id: model_id,
        version,
        kind: cfg.model_kind(),
        artefact_path,
        metadata: ModelMetadata {
            dp: cfg.dp().clone(),
            fairness: cfg.fairness().cloned(),
        },
    };

    let mut guard = registry()
        .lock()
        .map_err(|_| DeltaError::internal("model_registry_poisoned"))?;
    guard.insert(model.clone());

    Ok(model)
}

/// Load the requested model version or fall back to the latest when no version is provided.
pub fn load_model(id: &ModelId, version: Option<&VersionName>) -> DeltaResult<ModelVersion> {
    let guard = registry()
        .lock()
        .map_err(|_| DeltaError::internal("model_registry_poisoned"))?;
    let model = match version {
        Some(ver) if !ver.as_str().is_empty() => guard.get(id, ver),
        _ => guard.latest(id),
    };

    model.ok_or_else(|| DeltaError::model_missing("model_version"))
}

/// Export a compact model card JSON for auditability.
pub fn export_model_card(id: &ModelId) -> DeltaResult<String> {
    let guard = registry()
        .lock()
        .map_err(|_| DeltaError::internal("model_registry_poisoned"))?;
    let model = guard
        .latest(id)
        .ok_or_else(|| DeltaError::model_missing("model_version"))?;

    let fairness = model
        .metadata
        .fairness
        .as_ref()
        .map(|f| {
            format!(
                "{{\"delta_tpr\":{:.4},\"delta_fpr\":{:.4},\"delta_ppv\":{:.4}}}",
                f.delta_tpr, f.delta_fpr, f.delta_ppv
            )
        })
        .unwrap_or_else(|| "{}".to_string());

    let card = format!(
        "{{\"model_id\":\"{}\",\"version\":\"{}\",\"kind\":\"{}\",\"artefact\":\"{}\",\"dp\":{{\"enabled\":{},\"epsilon\":{:.4},\"delta\":{:.6},\"clip\":{:.4},\"noise_multiplier\":{:.4}}},\"fairness\":{}}}",
        crate::common::json::escape(model.id.as_str()),
        crate::common::json::escape(model.version.as_str()),
        crate::common::json::escape(model_kind_label(model.kind)),
        crate::common::json::escape(&model.artefact_path),
        if model.metadata.dp.enabled { "true" } else { "false" },
        model.metadata.dp.epsilon,
        model.metadata.dp.delta,
        model.metadata.dp.clip,
        model.metadata.dp.noise_multiplier,
        fairness
    );

    Ok(card)
}

fn make_model_id(dataset: &DatasetId, cfg_json: &str, kind: ModelKind) -> ModelId {
    let mut hasher = SimpleHash::new();
    hasher.update(dataset.as_str().as_bytes());
    hasher.update(cfg_json.as_bytes());
    hasher.update(model_kind_label(kind).as_bytes());
    ModelId::new(format!(
        "{}-{}",
        model_kind_label(kind),
        hasher.finish_hex()
    ))
}

fn model_kind_label(kind: ModelKind) -> &'static str {
    match kind {
        ModelKind::TabularLogistic => "tabular-logreg",
        ModelKind::TabularGradientBoosting => "tabular-gbdt",
        ModelKind::TextMiniLm => "text-minilm",
    }
}

fn enforce_dp(cfg: &TrainConfig) -> DeltaResult<()> {
    let dp = cfg.dp();
    if !dp.enabled {
        return Ok(());
    }

    if dp.epsilon > MAX_EPSILON + f32::EPSILON {
        return Err(DeltaError::policy_denied("dp_epsilon_exceeded"));
    }
    if dp.delta > MAX_DELTA {
        return Err(DeltaError::policy_denied("dp_delta_exceeded"));
    }
    if dp.clip <= 0.0 {
        return Err(DeltaError::policy_denied("dp_clip_invalid"));
    }
    if dp.noise_multiplier <= 0.0 {
        return Err(DeltaError::policy_denied("dp_noise_invalid"));
    }

    Ok(())
}

fn enforce_fairness(cfg: &TrainConfig) -> DeltaResult<()> {
    match cfg.fairness() {
        Some(report) => {
            check_fairness_delta(report.delta_tpr, MAX_DELTA_TPR, "delta_tpr_exceeded")?;
            check_fairness_delta(report.delta_fpr, MAX_DELTA_FPR, "delta_fpr_exceeded")?;
            check_fairness_delta(report.delta_ppv, MAX_DELTA_PPV, "delta_ppv_exceeded")?
        }
        None => return Err(DeltaError::policy_denied("fairness_report_missing")),
    }

    Ok(())
}

fn check_fairness_delta(value: f32, bound: f32, code: &'static str) -> DeltaResult<()> {
    if value > bound {
        Err(DeltaError::policy_denied(code))
    } else {
        Ok(())
    }
}

/// Helper used by tests to clear the in-memory registry.
#[cfg(test)]
pub(crate) fn reset_registry() {
    if let Ok(mut reg) = registry().lock() {
        reg.entries.clear();
        reg.latest.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fairness_gate_blocks_large_gaps() {
        reset_registry();
        let cfg = "{\"fairness\":{\"delta_tpr\":0.2,\"delta_fpr\":0.01,\"delta_ppv\":0.01},\"dp\":{\"enabled\":false}}";
        let err = train(DatasetId::new("ds-test"), cfg).unwrap_err();
        assert_eq!(
            err.code as u32,
            DeltaError::policy_denied("delta_tpr_exceeded").code as u32
        );
    }

    #[test]
    fn dp_gate_validates_parameters() {
        reset_registry();
        let cfg = "{\"fairness\":{\"delta_tpr\":0.01,\"delta_fpr\":0.01,\"delta_ppv\":0.01},\"dp\":{\"enabled\":true,\"epsilon\":4.0,\"delta\":0.00001,\"clip\":1.0,\"noise_multiplier\":1.0}}";
        let err = train(DatasetId::new("ds-test"), cfg).unwrap_err();
        assert_eq!(
            err.code as u32,
            DeltaError::policy_denied("dp_epsilon_exceeded").code as u32
        );
    }
}
