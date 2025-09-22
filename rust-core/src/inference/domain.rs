//! Domain definitions for inference requests, routing and explanations.
//!
//! The minimal helpers here avoid external dependencies by using the string
//! utilities provided in `common::json`.

use crate::common::error::{DeltaError, DeltaResult};
use crate::common::json;
use crate::training::domain::{ModelKind, ModelVersion};

/// Result of a single inference call, including WhyLog metadata for auditing.
#[derive(Clone, Debug)]
pub struct Prediction {
    pub json: String,
    pub latency_ms: u32,
    pub confidence: f32,
    pub whylog: WhyLog,
}

/// Lightweight WhyLog representation tracking saliency and canonical hash.
#[derive(Clone, Debug)]
pub struct WhyLog {
    pub hash: String,
    pub salient: Vec<String>,
    pub rationale: String,
}

/// Context accompanying an inference call.
#[derive(Clone, Debug)]
pub struct InferenceContext {
    pub purpose_id: String,
    pub subject_id: String,
    pub features_only: bool,
}

impl InferenceContext {
    pub fn new(
        purpose_id: impl Into<String>,
        subject_id: impl Into<String>,
        features_only: bool,
    ) -> Self {
        Self {
            purpose_id: purpose_id.into(),
            subject_id: subject_id.into(),
            features_only,
        }
    }
}

/// Router input summarising the request payload.
#[derive(Clone, Debug, Default)]
pub struct RouterContext {
    pub features_only: bool,
    pub text_length: usize,
}

impl RouterContext {
    pub fn from_payload(payload: &str, ctx: &InferenceContext) -> Self {
        let text_length = json::extract_string(payload, "text")
            .map(|s| s.chars().count())
            .unwrap_or(0);

        let input_flag = json::extract_bool(payload, "features_only").unwrap_or(false);
        let context_flag = json::extract_object(payload, "context")
            .and_then(|section| json::extract_bool(section, "features_only"))
            .unwrap_or(false);

        Self {
            features_only: ctx.features_only || input_flag || context_flag,
            text_length,
        }
    }
}

/// Target model family selected by the router.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RouteTarget {
    Tabular,
    Text,
}

impl RouteTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            RouteTarget::Tabular => "tabular",
            RouteTarget::Text => "text",
        }
    }
}

/// Reason the router picked a particular target.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RouteReason {
    FeatureOverride,
    LongText,
    DefaultTabular,
}

impl RouteReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            RouteReason::FeatureOverride => "features_only",
            RouteReason::LongText => "long_text",
            RouteReason::DefaultTabular => "default",
        }
    }
}

/// Router decision result.
#[derive(Copy, Clone, Debug)]
pub struct RouteDecision {
    pub target: RouteTarget,
    pub reason: RouteReason,
}

/// Router trait for SSM-style deterministic routing.
pub trait ModelRouter {
    fn route(&self, ctx: &RouterContext) -> RouteDecision;
}

/// Single-state-machine router implementing the documented heuristics.
#[derive(Default)]
pub struct SSMRouter;

impl SSMRouter {
    pub fn new() -> Self {
        Self
    }
}

impl ModelRouter for SSMRouter {
    fn route(&self, ctx: &RouterContext) -> RouteDecision {
        if ctx.features_only {
            return RouteDecision {
                target: RouteTarget::Tabular,
                reason: RouteReason::FeatureOverride,
            };
        }

        if ctx.text_length > 256 {
            return RouteDecision {
                target: RouteTarget::Text,
                reason: RouteReason::LongText,
            };
        }

        RouteDecision {
            target: RouteTarget::Tabular,
            reason: RouteReason::DefaultTabular,
        }
    }
}

/// Interface for consent lookups.
pub trait ConsentStore: Send + Sync {
    fn is_granted(&self, purpose_id: &str, subject_id: &str) -> DeltaResult<bool>;
}

/// Allow-all consent store placeholder until real storage is wired in.
#[derive(Default)]
pub struct AllowAllConsent;

impl ConsentStore for AllowAllConsent {
    fn is_granted(&self, _: &str, _: &str) -> DeltaResult<bool> {
        Ok(true)
    }
}

/// Engine response prior to final packaging into a prediction.
#[derive(Clone, Debug)]
pub struct EngineResponse {
    pub payload: String,
    pub confidence: f32,
    pub saliency: Vec<String>,
    pub rationale: String,
}

/// Engine abstraction to decouple service orchestration from concrete implementations.
pub trait InferEngine {
    fn kind(&self) -> RouteTarget;
    fn infer(&self, model: &ModelVersion, input: &str) -> DeltaResult<EngineResponse>;
}

/// Helper to map model kind to router targets for verification.
pub fn route_target_for_model(kind: ModelKind) -> RouteTarget {
    match kind {
        ModelKind::TabularLogistic | ModelKind::TabularGradientBoosting => RouteTarget::Tabular,
        ModelKind::TextMiniLm => RouteTarget::Text,
    }
}

/// Ensure the selected route matches the model family, otherwise return a fallback target.
pub fn validate_route(model: &ModelVersion, decision: RouteDecision) -> RouteTarget {
    let expected = route_target_for_model(model.kind);
    if expected == decision.target {
        decision.target
    } else {
        RouteTarget::Tabular
    }
}

/// Build an inference context from raw strings and optional JSON envelope.
pub fn build_context(purpose_id: &str, subject_id: &str, input: &str) -> InferenceContext {
    let features_only = json::extract_object(input, "context")
        .and_then(|ctx| json::extract_bool(ctx, "features_only"))
        .unwrap_or(false);

    InferenceContext::new(
        purpose_id.to_string(),
        subject_id.to_string(),
        features_only,
    )
}

/// Validate the active model matches the router decision or provide a fallback decision.
pub fn ensure_compatible(model: &ModelVersion, decision: RouteDecision) -> RouteDecision {
    let target = validate_route(model, decision);
    RouteDecision {
        target,
        reason: decision.reason,
    }
}

/// Utility to check consent and map the result to an error.
pub fn ensure_consent(store: &dyn ConsentStore, ctx: &InferenceContext) -> DeltaResult<()> {
    if store.is_granted(&ctx.purpose_id, &ctx.subject_id)? {
        Ok(())
    } else {
        Err(DeltaError::no_consent())
    }
}
