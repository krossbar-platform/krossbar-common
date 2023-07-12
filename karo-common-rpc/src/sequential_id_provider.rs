use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

/// Structure which provides sequential ID to send along RPC messages
#[derive(Clone)]
pub struct SequentialIdProvider {
    seq_no_counter: Arc<AtomicU64>,
}

impl SequentialIdProvider {
    pub fn new() -> Self {
        Self {
            seq_no_counter: Arc::new(0u64.into()),
        }
    }

    /// Get nex unique ID
    pub fn next_id(&self) -> u64 {
        self.seq_no_counter.fetch_add(1, Ordering::Release)
    }
}
