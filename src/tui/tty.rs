//! TTY handling for reading events when stdin is piped
//!
//! When stdin is piped (e.g., `tree | treemd`), we need to explicitly
//! read keyboard events from /dev/tty instead of stdin.
//!
//! Security considerations:
//! - Uses MaybeUninit for safer uninitialized memory handling
//! - Validates file descriptors before use
//! - Proper cleanup on error paths

use crossterm::event::{Event, poll, read};
use std::fs::File;
use std::io;
use std::time::Duration;

#[cfg(unix)]
use std::mem::MaybeUninit;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(unix)]
use std::sync::OnceLock;

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

/// Enable raw mode on the appropriate terminal device
///
/// If stdin is a TTY, enables raw mode on stdin (normal behavior).
/// If stdin is piped, opens /dev/tty and enables raw mode on it,
/// saving the original termios for full restoration in `disable_raw_mode`.
///
/// # Safety
/// Uses unsafe libc calls with proper MaybeUninit handling and fd validation.
#[cfg(unix)]
pub fn enable_raw_mode() -> io::Result<()> {
    if stdin_is_tty() {
        // Normal case: stdin is a TTY
        crossterm::terminal::enable_raw_mode()
    } else {
        // Stdin is piped - open /dev/tty and enable raw mode on it
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

        let tty_fd = tty.as_raw_fd();

        // Validate file descriptor
        if tty_fd < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid file descriptor for /dev/tty",
            ));
        }

        // Use MaybeUninit for safer uninitialized memory handling
        let mut orig_termios = MaybeUninit::<libc::termios>::uninit();

        // SAFETY: tcgetattr initializes the termios struct, fd is validated above
        unsafe {
            if libc::tcgetattr(tty_fd, orig_termios.as_mut_ptr()) != 0 {
                return Err(io::Error::last_os_error());
            }

            // SAFETY: tcgetattr succeeded, so orig_termios is now initialized
            let orig_termios = orig_termios.assume_init();

            // Save original termios for full restoration in disable_raw_mode
            let _ = SAVED_TERMIOS.set(orig_termios);

            // Enable raw mode on /dev/tty
            let mut termios = orig_termios;
            libc::cfmakeraw(&mut termios);

            if libc::tcsetattr(tty_fd, libc::TCSANOW, &termios) != 0 {
                return Err(io::Error::last_os_error());
            }
        }

        // File will close when dropped, but raw mode settings persist
        Ok(())
    }
}

#[cfg(not(unix))]
pub fn enable_raw_mode() -> io::Result<()> {
    crossterm::terminal::enable_raw_mode()
}

/// Disable raw mode on the appropriate terminal device
///
/// When stdin is piped, fully restores the original termios saved during
/// `enable_raw_mode`, ensuring all flags (IEXTEN, ICRNL, OPOST, etc.) are
/// properly restored.
///
/// # Safety
/// Uses unsafe libc calls with proper MaybeUninit handling.
#[cfg(unix)]
pub fn disable_raw_mode() -> io::Result<()> {
    if stdin_is_tty() {
        // Normal case: disable on stdin
        crossterm::terminal::disable_raw_mode()
    } else {
        // Stdin was piped - restore /dev/tty terminal settings
        let tty = File::options().read(true).write(true).open("/dev/tty").ok();

        if let Some(tty) = tty {
            let tty_fd = tty.as_raw_fd();

            // Validate file descriptor
            if tty_fd < 0 {
                return Ok(()); // Silently fail on cleanup
            }

            // SAFETY: tcsetattr restores the original termios, fd is validated
            unsafe {
                if let Some(orig) = SAVED_TERMIOS.get() {
                    // Full restoration of original termios
                    libc::tcsetattr(tty_fd, libc::TCSANOW, orig);
                } else {
                    // Fallback: best-effort restoration if original wasn't saved
                    let mut termios = MaybeUninit::<libc::termios>::uninit();
                    if libc::tcgetattr(tty_fd, termios.as_mut_ptr()) == 0 {
                        let mut termios = termios.assume_init();
                        termios.c_lflag |= libc::ICANON | libc::ECHO | libc::ISIG | libc::IEXTEN;
                        termios.c_iflag |= libc::ICRNL | libc::IXON;
                        termios.c_oflag |= libc::OPOST;
                        libc::tcsetattr(tty_fd, libc::TCSANOW, &termios);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(not(unix))]
pub fn disable_raw_mode() -> io::Result<()> {
    crossterm::terminal::disable_raw_mode()
}

/// Temporarily redirect stdin to /dev/tty, run a closure, then restore stdin.
///
/// # Safety
/// Uses unsafe libc calls for file descriptor manipulation with proper cleanup
/// on all error paths.
#[cfg(unix)]
fn with_tty_stdin<F, R>(f: F) -> io::Result<R>
where
    F: FnOnce() -> io::Result<R>,
{
    use std::os::unix::io::IntoRawFd;

    let stdin_fd = io::stdin().as_raw_fd();

    // SAFETY: isatty is safe to call with any fd
    if unsafe { libc::isatty(stdin_fd) } == 1 {
        return f();
    }

    // Stdin is piped — redirect to /dev/tty, run closure, restore
    // SAFETY: These libc calls manipulate file descriptors with proper error handling
    // and cleanup on all error paths
    unsafe {
        // Save current stdin
        let saved_stdin = libc::dup(0);
        if saved_stdin < 0 {
            return Err(io::Error::last_os_error());
        }

        // Open /dev/tty
        let tty = match File::options().read(true).write(true).open("/dev/tty") {
            Ok(f) => f,
            Err(e) => {
                libc::close(saved_stdin);
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "Cannot open /dev/tty: {}. Interactive mode requires a terminal.",
                        e
                    ),
                ));
            }
        };

        let tty_fd = tty.into_raw_fd();

        // Validate tty_fd
        if tty_fd < 0 {
            libc::close(saved_stdin);
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid file descriptor for /dev/tty",
            ));
        }

        // Redirect stdin to /dev/tty
        if libc::dup2(tty_fd, 0) < 0 {
            let err = io::Error::last_os_error();
            libc::close(tty_fd);
            libc::close(saved_stdin);
            return Err(err);
        }

        libc::close(tty_fd);

        // Run the closure (crossterm will use the redirected stdin)
        let result = f();

        // Restore original stdin (always, even if closure failed)
        libc::dup2(saved_stdin, 0);
        libc::close(saved_stdin);

        result
    }
}

/// Read an event from the terminal, handling piped stdin
///
/// On Unix systems, when stdin is piped, this temporarily redirects
/// stdin to /dev/tty for reading events, then restores it.
#[cfg(unix)]
pub fn read_event() -> io::Result<Event> {
    with_tty_stdin(read)
}

#[cfg(not(unix))]
pub fn read_event() -> io::Result<Event> {
    read()
}

/// Poll for an event with timeout, handling piped stdin
///
/// Returns true if an event is available, false if timeout occurred.
#[cfg(unix)]
pub fn poll_event(timeout: Duration) -> io::Result<bool> {
    with_tty_stdin(|| poll(timeout))
}

#[cfg(not(unix))]
pub fn poll_event(timeout: Duration) -> io::Result<bool> {
    poll(timeout)
}
