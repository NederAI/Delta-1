//! Filesystem repository for trained model artefacts.
//!
//! TODO: Validate artefact headers and enforce integrity checksums.
//! TODO: Implement retention policies for outdated versions.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

use crate::common::config::AppCfg;
use crate::common::error::{DeltaError, DeltaResult};

use super::domain::{ModelId, ModelRepo, ModelVersion, VersionName};

/// Persist model metadata and artefacts on the local filesystem.
pub struct FsModelRepo {
    root: PathBuf,
}

impl FsModelRepo {
    pub fn new(cfg: &AppCfg) -> Self {
        Self {
            root: PathBuf::from(&cfg.data_root).join("models"),
        }
    }

    fn ensure_dirs(&self, model: &ModelVersion) -> io::Result<()> {
        let dir = self
            .root
            .join(model.id.as_str())
            .join(model.version.as_str());
        fs::create_dir_all(dir)
    }

    fn artefact_path(&self, model: &ModelVersion) -> PathBuf {
        self.root
            .join(model.id.as_str())
            .join(model.version.as_str())
            .join("model.bin")
    }
}

impl ModelRepo for FsModelRepo {
    fn put_model(&self, model: &ModelVersion) -> DeltaResult<()> {
        self.ensure_dirs(model).map_err(|_| DeltaError::io())?;
        let path = self.artefact_path(model);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .map_err(|_| DeltaError::io())?;

        file.write_all(b"DELTA1")
            .and_then(|_| file.write_all(model.version.as_str().as_bytes()))
            .map_err(|_| DeltaError::io())?;
        // TODO: Write deterministic payload bytes once the training engine is ready.
        Ok(())
    }

    fn get_model(&self, _id: &ModelId, _version: &VersionName) -> DeltaResult<ModelVersion> {
        Err(DeltaError::not_implemented("FsModelRepo::get_model"))
    }
}

// TODO: Provide utilities to list available model versions in sorted order.
