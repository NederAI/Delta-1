//! Filesystem-backed repository for dataset metadata.
//!
//! TODO: Harden path handling and ensure directories are created with strict permissions.
//! TODO: Implement periodic compaction/cleanup routines when datasets are retired.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::common::config::AppCfg;
use crate::common::error::{DeltaError, DeltaResult};

use super::domain::{DataRepo, Dataset, DatasetId};

/// Filesystem repository rooted at `cfg.data_root`.
pub struct FsDataRepo {
    root: PathBuf,
}

impl FsDataRepo {
    pub fn new(cfg: &AppCfg) -> Self {
        Self {
            root: PathBuf::from(&cfg.data_root).join("datasets"),
        }
    }

    fn metadata_path(&self, id: DatasetId) -> PathBuf {
        self.root.join(format!("{}.meta", id.raw()))
    }

    fn ensure_dirs(&self) -> io::Result<()> {
        fs::create_dir_all(&self.root)
    }
}

impl DataRepo for FsDataRepo {
    fn put_dataset(&self, dataset: &Dataset) -> DeltaResult<()> {
        self.ensure_dirs().map_err(|_| DeltaError::io())?;
        let path = self.metadata_path(dataset.id);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .map_err(|_| DeltaError::io())?;

        writeln!(
            file,
            "id={};created_ms={};rows={};schema={}",
            dataset.id.raw(),
            dataset.created_ms,
            dataset.rows,
            dataset.schema.definition_json
        )
        .map_err(|_| DeltaError::io())?;

        // TODO: Persist additional metadata such as column stats and lineage references.
        Ok(())
    }

    fn get_dataset(&self, id: DatasetId) -> DeltaResult<Dataset> {
        let path = self.metadata_path(id);
        if !Path::new(&path).exists() {
            return Err(DeltaError::not_found("dataset"));
        }
        // TODO: Parse metadata files properly instead of returning a placeholder.
        Err(DeltaError::not_implemented("FsDataRepo::get_dataset"))
    }
}

// TODO: Add fs-based locking to coordinate concurrent writers.
