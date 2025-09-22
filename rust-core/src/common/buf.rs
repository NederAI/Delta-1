//! Simple reusable buffer helpers intended for IO heavy paths.
//!
//! TODO: Integrate with ingestion normalisers to avoid repeated allocations.
//! TODO: Assess whether a small object allocator would pay off for inference workloads.

/// Wrapper over `Vec<u8>` to make the intent explicit and centralise future pooling logic.
#[derive(Default, Debug)]
pub struct ReusableBuffer {
    inner: Vec<u8>,
}

impl ReusableBuffer {
    /// Create a buffer with a pre-allocated capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: Vec::with_capacity(cap),
        }
    }

    /// Reset the buffer so it can be reused without reallocating.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Mutable access to the underlying storage.
    pub fn as_mut(&mut self) -> &mut Vec<u8> {
        &mut self.inner
    }

    /// Immutable access to the underlying storage.
    pub fn as_slice(&self) -> &[u8] {
        &self.inner
    }

    /// Append data to the buffer.
    pub fn extend_from_slice(&mut self, data: &[u8]) {
        self.inner.extend_from_slice(data);
        // TODO: Guard against oversized payloads that could blow up memory usage.
    }
}
