pub mod cli;
pub mod effects;
pub mod engine;
pub mod utils;

use std::sync::atomic::{AtomicBool, Ordering};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// SIGINT is recorded and checked from the run loop so teardown (cursor
/// restore) happens through normal control flow — Drop alone would not run on
/// a raw signal exit (plan.md §8).
pub fn install_sigint_handler() {
    // SAFETY: signal(2) with a signal-safe handler that only stores a flag.
    unsafe {
        libc_signal(2 /* SIGINT */, handle_sigint as *const () as usize);
    }
}

extern "C" fn handle_sigint(_: i32) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// Restore default SIGPIPE so `ttfx ... | head` dies quietly like any Unix
/// tool instead of panicking on a broken pipe (Rust ignores SIGPIPE by default).
pub fn restore_sigpipe() {
    unsafe {
        libc_signal(13 /* SIGPIPE */, 0 /* SIG_DFL */);
    }
}

unsafe fn libc_signal(signum: i32, handler: usize) {
    unsafe extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(signum, handler);
    }
}
