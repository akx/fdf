use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct InterruptHandle {
    interrupted: Arc<AtomicBool>,
}

impl InterruptHandle {
    pub fn new() -> Self {
        Self {
            interrupted: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_interrupted(&self) {
        self.interrupted.store(true, Ordering::Relaxed);
    }

    pub fn is_interrupted(&self) -> bool {
        self.interrupted.load(Ordering::Relaxed)
    }

    pub fn check_and_reset_interrupt(&self) -> bool {
        self.interrupted.swap(false, Ordering::Relaxed)
    }
}

impl Default for InterruptHandle {
    fn default() -> Self {
        Self::new()
    }
}
