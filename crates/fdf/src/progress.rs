#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Starting file discovery with spinner (indeterminate)
    FindStarted,
    /// Update during file discovery
    FindProgress {
        n_dirs: u64,
        n_files: u64,
        n_bytes: u64,
    },
    /// Calculating statistics phase
    CalculatingStats { n_files: u64 },
    /// File discovery finished
    FindFinished,

    /// Starting hash computation
    HashStarted { total_files: u64 },
    /// Update during hash computation
    HashProgress {
        processed_files: u64,
        total_files: u64,
        completed_groups: u64,
        total_groups: u64,
    },
    /// Hash computation finished
    HashFinished,
}

/// Type alias for progress callback functions
pub type ProgressCallback<'a> = &'a mut dyn FnMut(ProgressEvent);
