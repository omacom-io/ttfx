pub mod cli;
pub mod effects;
pub mod engine;
pub mod utils;

use std::sync::atomic::{AtomicBool, Ordering};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static TERMINAL_RESIZED: AtomicBool = AtomicBool::new(false);

/// SIGINT is recorded and checked from the run loop so teardown (cursor
/// restore) happens through normal control flow — Drop alone would not run on
/// a raw signal exit (plan.md §8).
pub fn install_sigint_handler() {
    // SAFETY: signal(2) with a signal-safe handler that only stores a flag.
    unsafe {
        libc_signal(SIGINT, handle_sigint as *const () as usize);
    }
}

extern "C" fn handle_sigint(_: i32) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// Record terminal resizes so the CLI can rebuild effects whose canvas and
/// character positions were derived from the previous dimensions.
pub fn install_sigwinch_handler() {
    // SAFETY: signal(2) with a signal-safe handler that only stores a flag.
    unsafe {
        libc_signal(SIGWINCH, handle_sigwinch as *const () as usize);
    }
}

extern "C" fn handle_sigwinch(_: i32) {
    TERMINAL_RESIZED.store(true, Ordering::SeqCst);
}

/// Consume a pending terminal resize notification.
pub fn take_terminal_resize() -> bool {
    TERMINAL_RESIZED.swap(false, Ordering::SeqCst)
}

pub(crate) fn terminal_resize_pending() -> bool {
    TERMINAL_RESIZED.load(Ordering::SeqCst)
}

/// Restore default SIGPIPE so `ttfx ... | head` dies quietly like any Unix
/// tool instead of panicking on a broken pipe (Rust ignores SIGPIPE by default).
pub fn restore_sigpipe() {
    unsafe {
        libc_signal(SIGPIPE, SIG_DFL);
    }
}

const SIGINT: i32 = 2;
const SIGPIPE: i32 = 13;
/// 28 on Linux and on the BSDs, macOS included.
const SIGWINCH: i32 = 28;
const SIG_DFL: usize = 0;

unsafe fn libc_signal(signum: i32, handler: usize) {
    unsafe extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(signum, handler);
    }
}

/// Wait for the window to stop changing size before acting on a resize.
/// Dragging a window edge emits a SIGWINCH per step; rebuilding for each one
/// pins the animation at its opening frames for the whole drag and then starts
/// it over on release.
pub fn wait_for_resize_to_settle() {
    use std::time::{Duration, Instant};
    const QUIET: Duration = Duration::from_millis(40);
    const LIMIT: Duration = Duration::from_secs(2);

    let deadline = Instant::now() + LIMIT;
    let mut last = crate::engine::terminal::get_terminal_dimensions();
    while Instant::now() < deadline && !interrupted() {
        std::thread::sleep(QUIET);
        take_terminal_resize();
        let current = crate::engine::terminal::get_terminal_dimensions();
        if current == last {
            break;
        }
        last = current;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_resize_notifications_are_consumed() {
        take_terminal_resize();
        handle_sigwinch(SIGWINCH);
        assert!(terminal_resize_pending());
        assert!(take_terminal_resize());
        assert!(!terminal_resize_pending());
        assert!(!take_terminal_resize());
    }
}
