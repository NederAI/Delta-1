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
    /// Input failed validation.
    InvalidInput = 10,
    /// IO layer failure (filesystem, permissions, etc).
    Io = 20,
    /// Entity was not found in the requested backing store.
    NotFound = 30,
    /// Catch-all for bugs and unimplemented paths.
    Internal = 50,
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

    /// IO error helper.
    pub const fn io() -> Self {
        Self::new(DeltaCode::Io, "io")
    }

    /// Validation helper.
    pub const fn invalid(msg: &'static str) -> Self {
        Self::new(DeltaCode::InvalidInput, msg)
    }

    /// Not found helper.
    pub const fn not_found(msg: &'static str) -> Self {
        Self::new(DeltaCode::NotFound, msg)
    }

    /// Internal error helper.
    pub const fn internal(msg: &'static str) -> Self {
        Self::new(DeltaCode::Internal, msg)
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
        assert_eq!(DeltaCode::InvalidInput as u32, 10);
        assert_eq!(DeltaCode::Io as u32, 20);
        assert_eq!(DeltaCode::NotFound as u32, 30);
        assert_eq!(DeltaCode::Internal as u32, 50);
    }
}
