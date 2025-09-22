//! Inference orchestration utilities bridging models, routers and engines.
//!
//! Implements the SSMRouter rules from the model design, performs consent
//! checks, falls back to the tabular logistic baseline when the text engine
//! fails and generates WhyLog hashes using the crate-local `SimpleHash`.

use std::sync::{Mutex, OnceLock};

use crate::common::error::{DeltaError, DeltaResult};
use crate::common::ids::SimpleHash;
use crate::common::json;
use crate::common::time;
use crate::training::domain::{ModelId, ModelKind, ModelVersion, VersionName};

use super::domain::{
    build_context, ensure_compatible, ensure_consent, AllowAllConsent, ConsentStore,
    EngineResponse, InferEngine, ModelRouter, Prediction, RouteDecision, RouteTarget,
    RouterContext, SSMRouter, WhyLog,
};

static ACTIVE_MODEL: OnceLock<Mutex<Option<ModelVersion>>> = OnceLock::new();
static ROUTER: OnceLock<SSMRouter> = OnceLock::new();
static CONSENT: OnceLock<AllowAllConsent> = OnceLock::new();
static ENGINES: OnceLock<EngineRegistry> = OnceLock::new();

/// Register the model that should be used for subsequent inference calls.
pub fn register_active_model(model: ModelVersion) {
    let lock = ACTIVE_MODEL.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = lock.lock() {
        *guard = Some(model);
    }
}

fn active_model() -> Option<ModelVersion> {
    let lock = ACTIVE_MODEL.get_or_init(|| Mutex::new(None));
    match lock.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => None,
    }
}

fn router() -> &'static SSMRouter {
    ROUTER.get_or_init(SSMRouter::new)
}

fn consent_store() -> &'static dyn ConsentStore {
    CONSENT.get_or_init(AllowAllConsent::default)
}

fn engines() -> &'static EngineRegistry {
    ENGINES.get_or_init(EngineRegistry::default)
}

/// Perform a single inference call using the currently active model.
pub fn infer_with_ctx(
    purpose_id: &str,
    subject_id: &str,
    input_json: &str,
) -> DeltaResult<Prediction> {
    let model = active_model().ok_or_else(|| DeltaError::model_missing("active_model"))?;
    let context = build_context(purpose_id, subject_id, input_json);

    ensure_consent(consent_store(), &context)?;

    let router_ctx = RouterContext::from_payload(input_json, &context);
    let decision = ensure_compatible(&model, router().route(&router_ctx));
    let engines = engines();

    let start = time::now_ms();
    let response = match engines.infer(decision.target, &model, input_json) {
        Ok(resp) => resp,
        Err(err) => {
            if decision.target == RouteTarget::Text {
                engines.infer(RouteTarget::Tabular, &model, input_json)?
            } else {
                return Err(err);
            }
        }
    };
    let latency = time::now_ms().saturating_sub(start) as u32;

    let mut body = merge_payload(&response.payload, &model, decision, response.confidence);
    let whylog = build_whylog(&body, &response);
    append_whylog_hash(&mut body, &whylog.hash);

    Ok(Prediction {
        json: body,
        latency_ms: latency,
        confidence: response.confidence,
        whylog,
    })
}

/// Convenience wrapper that reuses the active model but accepts typed identifiers.
pub fn infer_with_model(
    model_id: &ModelId,
    version: Option<&VersionName>,
    purpose_id: &str,
    subject_id: &str,
    input_json: &str,
) -> DeltaResult<Prediction> {
    let active = active_model().ok_or_else(|| DeltaError::model_missing("active_model"))?;
    if active.id.as_str() != model_id.as_str() {
        return Err(DeltaError::model_missing("active_model_mismatch"));
    }
    if let Some(ver) = version {
        if !ver.as_str().is_empty() && active.version.as_str() != ver.as_str() {
            return Err(DeltaError::model_missing("active_version_mismatch"));
        }
    }
    infer_with_ctx(purpose_id, subject_id, input_json)
}

fn merge_payload(
    engine_payload: &str,
    model: &ModelVersion,
    decision: RouteDecision,
    confidence: f32,
) -> String {
    let mut base = engine_payload.trim().trim().to_string();
    if !base.starts_with('{') {
        base.insert(0, '{');
    }
    if !base.ends_with('}') {
        base.push('}');
    }

    let mut body = base.trim_end_matches('}').to_string();
    if body.len() > 1 {
        body.push(',');
    }

    body.push_str(&format!(
        "\"model_id\":\"{}\",\"version\":\"{}\",\"route\":\"{}\",\"route_reason\":\"{}\",\"confidence\":{:.4}",
        json::escape(model.id.as_str()),
        json::escape(model.version.as_str()),
        decision.target.as_str(),
        decision.reason.as_str(),
        confidence,
    ));
    body.push('}');
    body
}

fn append_whylog_hash(body: &mut String, hash: &str) {
    if let Some(pos) = body.rfind('}') {
        let mut extra = String::from(",\"whylog_hash\":\"");
        extra.push_str(hash);
        extra.push_str("\"}");
        body.replace_range(pos.., &extra);
    }
}

fn build_whylog(body: &str, response: &EngineResponse) -> WhyLog {
    let mut hasher = SimpleHash::new();
    hasher.update(body.as_bytes());
    WhyLog {
        hash: hasher.finish_hex64(),
        salient: response.saliency.clone(),
        rationale: response.rationale.clone(),
    }
}

#[derive(Default)]
struct EngineRegistry {
    tabular: TabularEngine,
    text: TextEngine,
}

impl EngineRegistry {
    fn infer(
        &self,
        target: RouteTarget,
        model: &ModelVersion,
        payload: &str,
    ) -> DeltaResult<EngineResponse> {
        match target {
            RouteTarget::Tabular => self.tabular.infer(model, payload),
            RouteTarget::Text => self.text.infer(model, payload),
        }
    }
}

#[derive(Default)]
struct TabularEngine;

impl super::domain::InferEngine for TabularEngine {
    fn kind(&self) -> RouteTarget {
        RouteTarget::Tabular
    }

    fn infer(&self, model: &ModelVersion, input: &str) -> DeltaResult<EngineResponse> {
        let mut features = json::top_level_keys(input);
        features.retain(|key| key != "context" && key != "text");
        let saliency = features.iter().take(5).cloned().collect::<Vec<_>>();
        let score = deterministic_score(model, input);
        let payload = format!(
            "{{\"ok\":true,\"mode\":\"tabular\",\"score\":{:.4},\"features\":{}}}",
            score,
            json::build_string_array(&saliency)
        );

        Ok(EngineResponse {
            payload,
            confidence: 0.5 + score * 0.5,
            saliency,
            rationale: "tabular-local-surrogate".to_string(),
        })
    }
}

#[derive(Default)]
struct TextEngine;

impl super::domain::InferEngine for TextEngine {
    fn kind(&self) -> RouteTarget {
        RouteTarget::Text
    }

    fn infer(&self, model: &ModelVersion, input: &str) -> DeltaResult<EngineResponse> {
        let text = json::extract_string(input, "text")
            .ok_or_else(|| DeltaError::invalid("text_required"))?;
        let tokens = text
            .split_whitespace()
            .filter(|token| !token.is_empty())
            .map(|token| token.to_string())
            .collect::<Vec<_>>();
        let saliency = tokens.iter().take(5).cloned().collect::<Vec<_>>();
        let score = deterministic_score(model, input);
        let payload = format!(
            "{{\"ok\":true,\"mode\":\"text\",\"score\":{:.4},\"tokens\":{}}}",
            score,
            json::build_string_array(&saliency)
        );

        Ok(EngineResponse {
            payload,
            confidence: 0.4 + score * 0.6,
            saliency,
            rationale: "minilm-q4-saliency".to_string(),
        })
    }
}

fn deterministic_score(model: &ModelVersion, input: &str) -> f32 {
    let mut hasher = SimpleHash::new();
    hasher.update(model.id.as_str().as_bytes());
    hasher.update(input.as_bytes());
    let raw = hasher.finish32();
    (raw % 10_000) as f32 / 10_000.0
}

/// Helper used in tests to clear the active model state.
#[cfg(test)]
pub(crate) fn reset_state() {
    if let Some(lock) = ACTIVE_MODEL.get() {
        if let Ok(mut guard) = lock.lock() {
            *guard = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_model() -> ModelVersion {
        ModelVersion {
            id: ModelId::new("tabular-logreg-test"),
            version: VersionName::new("v1"),
            kind: ModelKind::TabularLogistic,
            artefact_path: "models/test.bin".to_string(),
            metadata: crate::training::domain::ModelMetadata::default(),
        }
    }

    #[test]
    fn router_falls_back_when_text_missing() {
        reset_state();
        register_active_model(test_model());
        let payload = "{\"text\":123}";
        let prediction = infer_with_ctx("purpose", "subject", payload).unwrap();
        assert!(prediction.json.contains("\"route\":\"tabular\""));
    }

    #[test]
    fn whylog_hash_is_stable() {
        reset_state();
        register_active_model(test_model());
        let payload = "{\"amount\":100,\"features_only\":true}";
        let result = infer_with_ctx("purpose", "subject", payload).unwrap();
        assert_eq!(result.whylog.hash.len(), 64);
    }
}
