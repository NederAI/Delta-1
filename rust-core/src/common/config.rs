//! Runtime configuration loaded from environment and optional key-value files.
//!
//! TODO: Support layered configuration (file overrides env) without external dependencies.
//! TODO: Investigate hot-reload hooks and immutable snapshots for long-running workers.

use std::env;

/// Snapshot of configuration values consumed by the core.
#[derive(Clone, Debug)]
pub struct AppCfg {
    pub data_root: String,
    pub region: String,
    pub log_level: u8,
}

impl AppCfg {
    /// Create a configuration snapshot from the process environment.
    pub fn load() -> Self {
        fn env_or(key: &str, default: &str) -> String {
            env::var(key).unwrap_or_else(|_| default.to_string())
        }

        // TODO: Add validation for the directory structure, including permissions and ownership.
        // TODO: Merge values from a configurable key=value file to avoid large environment surfaces.

        Self {
            data_root: env_or("DELTA1_DATA_ROOT", "./data"),
            region: env_or("DELTA1_REGION", "eu"),
            log_level: env_or("DELTA1_LOG_LEVEL", "1").parse().unwrap_or(1),
        }
    }
}

/// Convenience wrapper kept for compatibility with the documentation examples.
pub fn load_cfg() -> AppCfg {
    AppCfg::load()
}
