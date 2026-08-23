//! Terminal control: raw mode, width, and resize notification.
//!
//! Unix only, like the Go original's build-tagged resize handling. Windows has
//! no SIGWINCH and needs the console API instead.

use std::io;
use std::os::fd::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};

static RESIZED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_winch(_: libc::c_int) {
    // The only async-signal-safe thing we need: flip a flag.
    RESIZED.store(true, Ordering::Relaxed);
}

/// Starts delivering SIGWINCH into [`take_resize`].
pub fn notify_resize() {
    // Cast via a pointer: casting a function item straight to an integer is
    // rejected by the function_casts_as_integer lint.
    let handler = on_winch as extern "C" fn(libc::c_int) as *const () as libc::sighandler_t;
    unsafe { libc::signal(libc::SIGWINCH, handler) };
}

/// Reports whether the terminal was resized since the last call.
pub fn take_resize() -> bool {
    RESIZED.swap(false, Ordering::Relaxed)
}

pub fn is_terminal(fd: RawFd) -> bool {
    unsafe { libc::isatty(fd) == 1 }
}

/// The terminal's column count, if it has one.
pub fn width(fd: RawFd) -> Option<u16> {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) };
    (rc == 0 && ws.ws_col > 0).then_some(ws.ws_col)
}

/// Puts the terminal in raw mode, restoring the previous settings on drop.
pub struct RawMode {
    fd: RawFd,
    saved: libc::termios,
}

impl RawMode {
    pub fn enable(fd: RawFd) -> io::Result<Self> {
        unsafe {
            let mut t: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(fd, &mut t) != 0 {
                return Err(io::Error::last_os_error());
            }
            let saved = t;
            libc::cfmakeraw(&mut t);
            if libc::tcsetattr(fd, libc::TCSANOW, &t) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Self { fd, saved })
        }
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        unsafe { libc::tcsetattr(self.fd, libc::TCSANOW, &self.saved) };
    }
}
