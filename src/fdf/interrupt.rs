use lazy_static::lazy_static;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

lazy_static! {
    pub static ref INTERRUPTED: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

pub fn configure_interrupt() {
    ctrlc::set_handler(move || {
        eprintln!("received Ctrl+C!");
        INTERRUPTED.store(true, Ordering::Relaxed);
    })
    .unwrap_or_else(|e| eprintln!("Error setting Ctrl-C handler: {}", e));
}

pub fn is_interrupted() -> bool {
    INTERRUPTED.load(Ordering::Relaxed)
}

pub fn check_and_reset_interrupt() -> bool {
    INTERRUPTED.swap(false, Ordering::Relaxed)
}
