use lazy_static::lazy_static;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

lazy_static! {
    pub static ref INTERRUPTED: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

pub fn set_interrupted() {
    INTERRUPTED.store(true, Ordering::Relaxed);
}

pub fn is_interrupted() -> bool {
    INTERRUPTED.load(Ordering::Relaxed)
}

pub fn check_and_reset_interrupt() -> bool {
    INTERRUPTED.swap(false, Ordering::Relaxed)
}
