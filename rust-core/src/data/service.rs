//! Service layer responsible for ingesting and normalising datasets.
//!
//! TODO: Plug in schema-aware validators once the specification is finalised.
//! TODO: Ensure ingestion is fully streaming to keep memory bounded for huge datasets.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::common::error::{DeltaError, DeltaResult};
use crate::common::ids::SimpleHash;
use crate::common::time;

use super::domain::{Dataset, DatasetId};

/// Ingest a file into the system, returning the assigned dataset identifier.
pub fn ingest_file(path: &str, schema_json: &str) -> DeltaResult<DatasetId> {
    // TODO: Add path sanitisation and root prefix enforcement to avoid traversal attacks.
    // TODO: Validate schema_json against allowed patterns before accepting it.
    let file = File::open(Path::new(path)).map_err(|_| DeltaError::io())?;
    let mut reader = BufReader::new(file);
    let mut hasher = SimpleHash::new();
    let mut line = String::new();
    let mut rows = 0u64;

    loop {
        line.clear();
        let read = reader.read_line(&mut line).map_err(|_| DeltaError::io())?;
        if read == 0 {
            break;
        }
        hasher.update(line.as_bytes());
        rows += 1;
        // TODO: Apply normalisation rules (trim, lowercase, PII strategies) before hashing.
    }

    let dataset_id = DatasetId::new(format!("ds-{}", hasher.finish_hex()));
    let dataset = Dataset::new(
        dataset_id.clone(),
        schema_json.to_string(),
        time::now_ms(),
        rows,
    );

    // TODO: Persist dataset metadata via DataRepo once wiring is in place.

    Ok(dataset.id)
}

/// Export a placeholder datasheet for the given dataset identifier.
pub fn export_datasheet(dataset_id: &DatasetId) -> DeltaResult<String> {
    let sheet = format!(
        "{{\"dataset_id\":\"{}\",\"schema\":\"inline\",\"retention_days\":30,\"created_ms\":{}}}",
        crate::common::json::escape(dataset_id.as_str()),
        time::now_ms()
    );

    Ok(sheet)
}

// TODO: Provide a dry-run API for validation without persistence side-effects.
