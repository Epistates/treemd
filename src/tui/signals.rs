//! Signals that end the session, handled so the terminal is restored.
//!
//! Left at their defaults, SIGTERM, SIGHUP (a closed terminal) and SIGINT
//! killed the process with raw mode, the alternate screen and mouse
//! reporting still on, so the shell received mouse escapes as input.
//!
//! A signal only sets a flag that the event loop checks, so the session
//! ends through its normal exit path. The loop cannot check while the UI
//! thread is busy (laying out a huge section, decoding an image), so if it
//! has not exited shortly after SIGTERM or SIGHUP, the terminal is restored
//! and the process exits from here.
//!
//! While an external editor runs, the terminal is in its normal mode and
//! Ctrl+C sends SIGINT to treemd as well as the editor. That one is the
//! editor's: it is dropped when the editor returns, as `system(3)` and git
//! do, instead of ending treemd and losing unsaved edits.

use std::sync::atomic::{AtomicBool, Ordering};

static TERMINATE: AtomicBool = AtomicBool::new(false);
static INTERRUPT: AtomicBool = AtomicBool::new(false);

/// How long the event loop has to exit on its own after SIGTERM or SIGHUP.
#[cfg(unix)]
const GRACE: std::time::Duration = std::time::Duration::from_secs(2);

/// Start handling the signals. Call before the terminal leaves its normal
/// mode, so there is no window in which a signal leaves it changed.
#[cfg(unix)]
pub fn install() {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};

    let Ok(mut signals) = signal_hook::iterator::Signals::new([SIGTERM, SIGHUP, SIGINT, SIGQUIT])
    else {
        return;
    };
    std::thread::Builder::new()
        .name("treemd-signals".into())
        .spawn(move || {
            for signal in signals.forever() {
                if signal == SIGINT || signal == SIGQUIT {
                    INTERRUPT.store(true, Ordering::Relaxed);
                    continue;
                }
                TERMINATE.store(true, Ordering::Relaxed);
                std::thread::spawn(move || {
                    std::thread::sleep(GRACE);
                    crate::tui::tty::restore();
                    std::process::exit(128 + signal);
                });
            }
        })
        .ok();
}

#[cfg(not(unix))]
pub fn install() {}

/// Whether a signal has asked the session to end.
pub fn should_exit() -> bool {
    TERMINATE.load(Ordering::Relaxed) || INTERRUPT.load(Ordering::Relaxed)
}

/// Drop an interrupt that arrived while an external program had the
/// terminal: it was meant for that program.
pub fn forget_interrupt() {
    INTERRUPT.store(false, Ordering::Relaxed);
}
