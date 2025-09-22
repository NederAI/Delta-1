//! Error handling primitives shared across the core.
//!
//! TODO: Expand with richer diagnostics once telemetry conventions land.
//! TODO: Consider moving to a compact error table generated at build time for ABI stability.

/// Stable error codes that cross the FFI boundary.
#[repr(u32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DeltaCode {
    /// Success code used as a sentinel.
    Ok = 0,
    /// Consent check failed for the provided subject/purpose.
    NoConsent = 1,
    /// Policy guardrail denied the operation.
    PolicyDenied = 2,
    /// Requested model artefact was not available.
    ModelMissing = 3,
    /// Input failed validation.
    InvalidInput = 4,
    /// Catch-all for bugs and unimplemented paths.
    Internal = 5,
}

/// Canonical error type for the core.
#[derive(Copy, Clone, Debug)]
pub struct DeltaError {
    /// Machine parsable error code.
    pub code: DeltaCode,
    /// Developer facing message (keep &'static str for FFI safety).
    pub msg: &'static str,
}

/// Result alias used throughout the crate.
pub type DeltaResult<T> = Result<T, DeltaError>;

impl DeltaError {
    /// Create a new error with the provided code and message.
    pub const fn new(code: DeltaCode, msg: &'static str) -> Self {
        Self { code, msg }
    }

    /// Validation helper.
    pub const fn invalid(msg: &'static str) -> Self {
        Self::new(DeltaCode::InvalidInput, msg)
    }

    /// Policy helper.
    pub const fn policy_denied(msg: &'static str) -> Self {
        Self::new(DeltaCode::PolicyDenied, msg)
    }

    /// Consent helper.
    pub const fn no_consent() -> Self {
        Self::new(DeltaCode::NoConsent, "no_consent")
    }

    /// Model missing helper.
    pub const fn model_missing(msg: &'static str) -> Self {
        Self::new(DeltaCode::ModelMissing, msg)
    }

    /// Internal error helper.
    pub const fn internal(msg: &'static str) -> Self {
        Self::new(DeltaCode::Internal, msg)
    }

    /// IO error helper (mapped to internal until dedicated code exists).
    pub const fn io() -> Self {
        Self::internal("io")
    }

    /// Temporary helper until the real implementation lands.
    pub const fn not_implemented(feature: &'static str) -> Self {
        // TODO: Consider a dedicated error code for not-implemented once the ABI versioning plan lands.
        Self::new(DeltaCode::Internal, feature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable() {
        assert_eq!(DeltaCode::Ok as u32, 0);
        assert_eq!(DeltaCode::NoConsent as u32, 1);
        assert_eq!(DeltaCode::PolicyDenied as u32, 2);
        assert_eq!(DeltaCode::ModelMissing as u32, 3);
        assert_eq!(DeltaCode::InvalidInput as u32, 4);
        assert_eq!(DeltaCode::Internal as u32, 5);
    }
}
