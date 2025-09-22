//! Lightweight logging utilities emitting JSON lines.
//!
//! TODO: Wire structured context (dataset/model identifiers) into each log entry.
//! TODO: Provide pluggable sinks once we move beyond stdout/stderr for observability.

/// Emit a JSON line matching the documented schema.
pub fn log_json(level: &str, module: &str, event: &str, code: u32, dur_ms: u128) {
    let ts = crate::common::time::now_ms();
    println!(
        "{{\"ts\":{ts},\"level\":\"{level}\",\"mod\":\"{module}\",\"ev\":\"{event}\",\"code\":{code},\"dur_ms\":{dur_ms}}}"
    );
    // TODO: Add sampling and rate-limiting to prevent flooding when ingesting large batches.
}
