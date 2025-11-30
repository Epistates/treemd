//! TTY handling for reading events when stdin is piped
//!
//! When stdin is piped (e.g., `tree | treemd`), we need to explicitly
//! read keyboard events from /dev/tty instead of stdin.
//!
//! Security considerations:
//! - Uses MaybeUninit for safer uninitialized memory handling
//! - Validates file descriptors before use
//! - Proper cleanup on error paths

use crossterm::event::{Event, read};
use std::io;
use std::fs::File;

#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(unix)]
use std::mem::MaybeUninit;

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

/// Enable raw mode on the appropriate terminal device
///
/// If stdin is a TTY, enables raw mode on stdin (normal behavior).
/// If stdin is piped, opens /dev/tty and enables raw mode on it.
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
                    format!("Cannot open /dev/tty: {}. Interactive mode requires a terminal.", e)
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
/// # Safety
/// Uses unsafe libc calls with proper MaybeUninit handling.
#[cfg(unix)]
pub fn disable_raw_mode() -> io::Result<()> {
    if stdin_is_tty() {
        // Normal case: disable on stdin
        crossterm::terminal::disable_raw_mode()
    } else {
        // Stdin was piped - restore /dev/tty terminal settings
        let tty = File::options()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .ok();

        if let Some(tty) = tty {
            let tty_fd = tty.as_raw_fd();

            // Validate file descriptor
            if tty_fd < 0 {
                return Ok(()); // Silently fail on cleanup
            }

            // Use MaybeUninit for safer uninitialized memory handling
            let mut termios = MaybeUninit::<libc::termios>::uninit();

            // SAFETY: tcgetattr initializes the termios struct, fd is validated
            unsafe {
                if libc::tcgetattr(tty_fd, termios.as_mut_ptr()) == 0 {
                    // SAFETY: tcgetattr succeeded, so termios is now initialized
                    let mut termios = termios.assume_init();
                    // Reset to cooked mode
                    termios.c_lflag |= libc::ICANON | libc::ECHO | libc::ISIG;
                    libc::tcsetattr(tty_fd, libc::TCSANOW, &termios);
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

/// Read an event from the terminal, handling piped stdin
///
/// On Unix systems, when stdin is piped, this temporarily redirects
/// stdin to /dev/tty for reading events, then restores it.
///
/// # Safety
/// Uses unsafe libc calls for file descriptor manipulation with proper cleanup.
#[cfg(unix)]
pub fn read_event() -> io::Result<Event> {
    use std::os::unix::io::{AsRawFd, IntoRawFd};

    // Check if stdin is a TTY
    let stdin_fd = io::stdin().as_raw_fd();

    // SAFETY: isatty is safe to call with any fd
    if unsafe { libc::isatty(stdin_fd) } == 1 {
        // Stdin is a TTY, use normal event reading
        return read();
    }

    // Stdin is piped - we need to read from /dev/tty
    // Strategy: dup stdin, open /dev/tty, dup2 it to fd 0, read event, restore stdin

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

        // Now read the event (crossterm will use the redirected stdin)
        let result = read();

        // Restore original stdin (always, even if read failed)
        libc::dup2(saved_stdin, 0);
        libc::close(saved_stdin);

        result
    }
}

#[cfg(not(unix))]
pub fn read_event() -> io::Result<Event> {
    read()
}
