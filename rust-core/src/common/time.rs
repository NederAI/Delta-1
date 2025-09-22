//! Simple time helpers used by multiple services.
//!
//! TODO: Evaluate monotonic clocks for latency tracking vs wall-clock requirements.
//! TODO: Consider exposing high-resolution timers for CPU intensive sections.

use std::time::{SystemTime, UNIX_EPOCH};

/// Current timestamp in milliseconds since the Unix epoch.
pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
