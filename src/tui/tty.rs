//! TTY handling for reading events when stdin is piped.
//!
//! When stdin is piped (e.g. `tree | treemd`), keyboard events still need to
//! come from a real terminal. We open `/dev/tty` once and redirect fd 0 to it
//! for the duration of the TUI session, then restore the original stdin on
//! shutdown. Crossterm's `read`/`poll` then work directly without per-call
//! file-descriptor swaps.

use crossterm::event::{Event, poll, read};
use std::io;
use std::time::Duration;

#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::mem::MaybeUninit;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, IntoRawFd};
#[cfg(unix)]
use std::sync::{Mutex, OnceLock};

/// Check if stdin is a TTY
#[cfg(unix)]
fn stdin_is_tty() -> bool {
    let stdin_fd = io::stdin().as_raw_fd();
    unsafe { libc::isatty(stdin_fd) == 1 }
}

#[cfg(not(unix))]
fn stdin_is_tty() -> bool {
    use std::io::IsTerminal;
    io::stdin().is_terminal()
}

/// Saved original termios for full restoration when stdin is piped.
#[cfg(unix)]
static SAVED_TERMIOS: OnceLock<libc::termios> = OnceLock::new();

/// State of the one-shot stdin → /dev/tty redirect.
#[cfg(unix)]
struct StdinRedirect {
    /// Original stdin fd (the read end of the pipe), saved via `dup` so we can
    /// restore it on shutdown.
    saved_stdin: libc::c_int,
}

#[cfg(unix)]
static STDIN_REDIRECT: OnceLock<Mutex<Option<StdinRedirect>>> = OnceLock::new();

#[cfg(unix)]
fn redirect_state() -> &'static Mutex<Option<StdinRedirect>> {
    STDIN_REDIRECT.get_or_init(|| Mutex::new(None))
}

/// Open `/dev/tty` and dup it onto fd 0, saving the original stdin so it can
/// be restored at shutdown. Idempotent — does nothing if already redirected.
#[cfg(unix)]
fn redirect_stdin_to_tty_once() -> io::Result<()> {
    let state = redirect_state();
    let mut guard = state.lock().expect("STDIN_REDIRECT mutex poisoned");
    if guard.is_some() {
        return Ok(());
    }

    let tty = File::options()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Cannot open /dev/tty: {}. Interactive mode requires a terminal.",
                    e
                ),
            )
        })?;

    let tty_fd = tty.into_raw_fd();
    if tty_fd < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid file descriptor for /dev/tty",
        ));
    }

    // SAFETY: dup/dup2/close are standard FD operations with checked returns.
    unsafe {
        let saved_stdin = libc::dup(0);
        if saved_stdin < 0 {
            libc::close(tty_fd);
            return Err(io::Error::last_os_error());
        }

        if libc::dup2(tty_fd, 0) < 0 {
            let err = io::Error::last_os_error();
            libc::close(tty_fd);
            libc::close(saved_stdin);
            return Err(err);
        }

        libc::close(tty_fd);

        *guard = Some(StdinRedirect { saved_stdin });
    }

    Ok(())
}

/// Restore the original stdin if we redirected it. Best-effort.
#[cfg(unix)]
fn restore_stdin_redirect() {
    let state = redirect_state();
    let Ok(mut guard) = state.lock() else { return };
    if let Some(redirect) = guard.take() {
        // SAFETY: redirect.saved_stdin came from dup(0); restore it then close.
        unsafe {
            libc::dup2(redirect.saved_stdin, 0);
            libc::close(redirect.saved_stdin);
        }
    }
}

/// Enable raw mode on the appropriate terminal device.
///
/// If stdin is piped, opens `/dev/tty`, redirects fd 0 to it once, and enables
/// raw mode there. The original stdin is saved so it can be restored on
/// shutdown.
#[cfg(unix)]
pub fn enable_raw_mode() -> io::Result<()> {
    if stdin_is_tty() {
        return crossterm::terminal::enable_raw_mode();
    }

    // Stdin is piped — point fd 0 at /dev/tty for the rest of the session.
    redirect_stdin_to_tty_once()?;

    // Now read termios from fd 0 (which is /dev/tty) and put it in raw mode.
    let mut orig_termios = MaybeUninit::<libc::termios>::uninit();
    unsafe {
        if libc::tcgetattr(0, orig_termios.as_mut_ptr()) != 0 {
            return Err(io::Error::last_os_error());
        }

        let orig = orig_termios.assume_init();
        let _ = SAVED_TERMIOS.set(orig);

        let mut termios = orig;
        libc::cfmakeraw(&mut termios);

        if libc::tcsetattr(0, libc::TCSANOW, &termios) != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

#[cfg(not(unix))]
pub fn enable_raw_mode() -> io::Result<()> {
    crossterm::terminal::enable_raw_mode()
}

/// Disable raw mode and restore the original stdin (if it was redirected).
#[cfg(unix)]
pub fn disable_raw_mode() -> io::Result<()> {
    if stdin_is_tty() {
        return crossterm::terminal::disable_raw_mode();
    }

    // Restore termios on fd 0 (currently /dev/tty), then swap fd 0 back to
    // the original stdin.
    unsafe {
        if let Some(orig) = SAVED_TERMIOS.get() {
            libc::tcsetattr(0, libc::TCSANOW, orig);
        } else {
            // Fallback: best-effort restoration of canonical mode.
            let mut termios = MaybeUninit::<libc::termios>::uninit();
            if libc::tcgetattr(0, termios.as_mut_ptr()) == 0 {
                let mut termios = termios.assume_init();
                termios.c_lflag |= libc::ICANON | libc::ECHO | libc::ISIG | libc::IEXTEN;
                termios.c_iflag |= libc::ICRNL | libc::IXON;
                termios.c_oflag |= libc::OPOST;
                libc::tcsetattr(0, libc::TCSANOW, &termios);
            }
        }
    }

    restore_stdin_redirect();
    Ok(())
}

#[cfg(not(unix))]
pub fn disable_raw_mode() -> io::Result<()> {
    crossterm::terminal::disable_raw_mode()
}

/// Best-effort terminal restore: leave alternate screen and disable raw mode.
///
/// Safe to call from a panic hook or during shutdown.
pub fn restore() {
    use crossterm::ExecutableCommand;
    use crossterm::event::DisableMouseCapture;
    use crossterm::terminal::LeaveAlternateScreen;
    use std::io::stdout;

    // Each step is best-effort — some terminals reject mouse capture commands,
    // and we still want to leave the altscreen and drop raw mode.
    let _ = stdout().execute(DisableMouseCapture);
    let _ = stdout().execute(LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

/// Install a panic hook that restores the terminal before propagating the panic.
///
/// Mirrors `ratatui::init`'s behavior but uses our piped-stdin-aware restore.
pub fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        prev(info);
    }));
}

/// Read a terminal event. fd 0 already points at /dev/tty when stdin was piped
/// (set up by `enable_raw_mode`), so this is just a thin pass-through.
pub fn read_event() -> io::Result<Event> {
    read()
}

/// Poll for a terminal event with timeout. Same one-shot redirect rationale as
/// `read_event`.
pub fn poll_event(timeout: Duration) -> io::Result<bool> {
    poll(timeout)
}
