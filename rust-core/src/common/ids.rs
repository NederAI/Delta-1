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

    /// Finalise the hash and return an 8-character lowercase hex string.
    pub fn finish_hex(&self) -> String {
        format!("{self:08x}", self = self.0)
    }

    /// Finalise the hash and return a 64-character lowercase hex string.
    pub fn finish_hex64(&self) -> String {
        let mut state = self.0;
        let mut out = String::with_capacity(64);
        for i in 0..8 {
            state = state.rotate_left(5).wrapping_add(0x9E37_79B9)
                ^ ((i as u32).wrapping_mul(0x85EB_CA6B));
            out.push_str(&format!("{state:08x}"));
        }
        out
    }
}

impl Default for SimpleHash {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Consider exposing helper methods that yield hex strings for log readability.
