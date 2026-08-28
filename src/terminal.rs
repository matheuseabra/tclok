use std::ffi::{c_int, c_long, c_ulong};
use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::layout::TerminalSize;

const STDOUT_FILENO: c_int = 1;
const STDIN_FILENO: c_int = 0;
const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;
const SIGWINCH: c_int = 28;

#[cfg(target_os = "linux")]
const TIOCGWINSZ: c_ulong = 0x5413;
#[cfg(target_os = "macos")]
const TIOCGWINSZ: c_ulong = 0x4008_7468;

static RESIZE_PENDING: AtomicBool = AtomicBool::new(false);
static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

#[repr(C)]
struct WinSize {
    rows: u16,
    columns: u16,
    x_pixels: u16,
    y_pixels: u16,
}

#[repr(C)]
struct TimeSpec {
    seconds: c_long,
    nanoseconds: c_long,
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct Termios {
    input_flags: c_ulong,
    output_flags: c_ulong,
    control_flags: c_ulong,
    local_flags: c_ulong,
    control_characters: [u8; 20],
    input_speed: c_ulong,
    output_speed: c_ulong,
}

unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn signal(signal: c_int, handler: usize) -> usize;
    fn nanosleep(requested: *const TimeSpec, remaining: *mut TimeSpec) -> c_int;
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn tcgetattr(file_descriptor: c_int, attributes: *mut Termios) -> c_int;
    fn tcsetattr(file_descriptor: c_int, action: c_int, attributes: *const Termios) -> c_int;
    fn read(file_descriptor: c_int, buffer: *mut u8, length: usize) -> isize;
}

extern "C" fn handle_signal(signal_number: c_int) {
    if signal_number == SIGWINCH {
        RESIZE_PENDING.store(true, Ordering::Relaxed);
    } else {
        STOP_REQUESTED.store(true, Ordering::Relaxed);
    }
}

pub fn stdout_is_terminal() -> bool {
    io::stdout().is_terminal()
}

pub fn terminal_size() -> Option<TerminalSize> {
    terminal_window_size().map(|size| TerminalSize {
        columns: size.columns,
        rows: size.rows,
    })
}

pub fn terminal_pixel_size() -> Option<(u16, u16)> {
    terminal_window_size()
        .and_then(|size| {
            (size.x_pixels > 0 && size.y_pixels > 0).then_some((size.x_pixels, size.y_pixels))
        })
        .or_else(query_terminal_pixel_size)
}

#[cfg(not(target_os = "macos"))]
fn query_terminal_pixel_size() -> Option<(u16, u16)> {
    None
}

#[cfg(target_os = "macos")]
fn query_terminal_pixel_size() -> Option<(u16, u16)> {
    const ICANON: c_ulong = 0x0000_0100;
    const ECHO: c_ulong = 0x0000_0008;
    const VMIN: usize = 16;
    const VTIME: usize = 17;
    const TCSANOW: c_int = 0;
    let mut original = std::mem::MaybeUninit::<Termios>::uninit();
    // SAFETY: stdin is a valid terminal file descriptor in interactive mode;
    // `original` points to enough writable space for the Darwin termios ABI.
    if unsafe { tcgetattr(STDIN_FILENO, original.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: `tcgetattr` succeeded above, so the structure is initialized.
    let original = unsafe { original.assume_init() };
    let mut query_mode = original;
    query_mode.local_flags &= !(ICANON | ECHO);
    query_mode.control_characters[VMIN] = 0;
    query_mode.control_characters[VTIME] = 1;
    // SAFETY: `query_mode` is based on a valid termios value and differs only
    // in local input behavior for the bounded query below.
    if unsafe { tcsetattr(STDIN_FILENO, TCSANOW, &query_mode) } != 0 {
        return None;
    }
    let mut response = [0_u8; 64];
    // SAFETY: stdout is valid; the bytes are the standardized CSI 14t query.
    let mut stdout = io::stdout().lock();
    let write_result = stdout.write_all(b"\x1b[14t");
    let _ = stdout.flush();
    let length = if write_result.is_ok() {
        // SAFETY: `response` is a valid writable buffer and VTIME bounds this
        // read to roughly one tenth of a second when Ghostty has no reply.
        unsafe { read(STDIN_FILENO, response.as_mut_ptr(), response.len()) }
    } else {
        -1
    };
    // SAFETY: restore the exact terminal settings captured before the query.
    let _ = unsafe { tcsetattr(STDIN_FILENO, TCSANOW, &original) };
    (length > 0)
        .then(|| parse_pixel_response(&response[..length as usize]))
        .flatten()
}

#[cfg(target_os = "macos")]
fn parse_pixel_response(bytes: &[u8]) -> Option<(u16, u16)> {
    let response = std::str::from_utf8(bytes).ok()?;
    let start = response.find("\x1b[4;")? + 4;
    let mut fields = response[start..response[start..].find('t')? + start].split(';');
    let height = fields.next()?.parse().ok()?;
    let width = fields.next()?.parse().ok()?;
    (width > 0 && height > 0).then_some((width, height))
}

fn terminal_window_size() -> Option<WinSize> {
    let mut size = WinSize {
        rows: 0,
        columns: 0,
        x_pixels: 0,
        y_pixels: 0,
    };
    // SAFETY: stdout file descriptor and `size` are valid for `ioctl`.
    let result = unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut size) };
    (result == 0 && size.columns > 0 && size.rows > 0).then_some(size)
}

pub fn take_resize_notification() -> bool {
    RESIZE_PENDING.swap(false, Ordering::Relaxed)
}

pub fn stop_requested() -> bool {
    STOP_REQUESTED.load(Ordering::Relaxed)
}

pub fn sleep_until_signal_or_timeout(duration: Duration) {
    let seconds = duration.as_secs().min(c_long::MAX as u64) as c_long;
    let nanoseconds = c_long::from(duration.subsec_nanos() as i32);
    let request = TimeSpec {
        seconds,
        nanoseconds,
    };
    // SAFETY: `request` is a valid timespec. EINTR is expected when a handled
    // resize or shutdown signal arrives, and is intentionally ignored here.
    unsafe {
        nanosleep(&request, std::ptr::null_mut());
    }
}

pub struct TerminalSession {
    previous_handlers: [(c_int, usize); 3],
}

impl TerminalSession {
    pub fn enter() -> io::Result<Self> {
        let previous_handlers = install_handlers()?;
        let session = Self { previous_handlers };
        let mut stdout = io::stdout().lock();
        if let Err(error) = stdout.write_all(b"\x1b[?1049h\x1b[?25l\x1b[0m") {
            drop(session);
            return Err(error);
        }
        stdout.flush()?;
        Ok(session)
    }
}

fn install_handlers() -> io::Result<[(c_int, usize); 3]> {
    let mut installed = Vec::with_capacity(3);
    for signal_number in [SIGWINCH, SIGINT, SIGTERM] {
        match install_handler(signal_number) {
            Ok(previous) => installed.push(previous),
            Err(error) => {
                for (installed_signal, previous) in installed.into_iter().rev() {
                    // SAFETY: Each disposition came from a successful `signal`
                    // call earlier in this transaction.
                    unsafe {
                        signal(installed_signal, previous);
                    }
                }
                return Err(error);
            }
        }
    }
    Ok([installed[0], installed[1], installed[2]])
}

fn install_handler(signal_number: c_int) -> io::Result<(c_int, usize)> {
    // SAFETY: `handle_signal` has the C signal ABI and only stores to lock-free
    // atomics. `signal` returns the prior disposition for restoration.
    let previous = unsafe { signal(signal_number, handle_signal as *const () as usize) };
    if previous == usize::MAX {
        Err(io::Error::last_os_error())
    } else {
        Ok((signal_number, previous))
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = io::stdout()
            .lock()
            .write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l");
        let _ = io::stdout().lock().flush();
        for (signal_number, previous) in self.previous_handlers {
            // SAFETY: `previous` is the disposition returned by `signal` for
            // the same signal number during `enter`.
            unsafe {
                signal(signal_number, previous);
            }
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn parses_the_standard_pixel_size_reply() {
        assert_eq!(parse_pixel_response(b"\x1b[4;900;1440t"), Some((1440, 900)));
    }
}
