//! Deterministic hash helpers for datasets, models and other identifiers.
//!
//! TODO: Evaluate swapping to 64-bit hashes for lower collision probabilities.
//! TODO: Provide a streaming API for incremental normalisation pipelines.

/// Extremely small non-cryptographic hash used for dataset identifiers.
#[derive(Copy, Clone, Debug)]
pub struct SimpleHash(u32);

impl SimpleHash {
    /// Create a new hash state with the FNV offset basis.
    pub fn new() -> Self {
        Self(216_613_626_1)
    }

    /// Feed bytes into the hash function.
    pub fn update(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ (*b as u32)).wrapping_mul(16_777_619);
        }
    }

    /// Finalise the hash and return a 32-bit value.
    pub fn finish32(&self) -> u32 {
        self.0
    }
}

impl Default for SimpleHash {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Consider exposing helper methods that yield hex strings for log readability.
