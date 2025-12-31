//! Print framework

use crate::os::{Fd, STDERR, STDOUT};
use crate::types::pid_t;
use core::ffi::CStr;
use core::sync::atomic::{AtomicU8, Ordering};

// ANSI color codes
const RESET: &[u8] = b"\x1b[0m";
const RED: &[u8] = b"\x1b[31m";
const GREEN: &[u8] = b"\x1b[32m";
const YELLOW: &[u8] = b"\x1b[33m";
const CYAN: &[u8] = b"\x1b[36m";
const LIGHT_GRAY: &[u8] = b"\x1b[2m";
const DARK_GRAY: &[u8] = b"\x1b[90m";
const GRAY_250: &[u8] = b"\x1b[38;5;250m";
const GRAY_248: &[u8] = b"\x1b[38;5;248m";
const GRAY_246: &[u8] = b"\x1b[38;5;246m";
const GRAY_244: &[u8] = b"\x1b[38;5;244m";

/// Color options for terminal output
#[derive(Clone, Copy)]
pub enum Color {
    Okay,
    Warning,
    Error,
    Transition,
    Service,
    Path,
    Dim,
    Glue,
    NotFound,
    TimeDay,
    TimeHour,
    TimeMinute,
    TimeSecond,
}

impl Color {
    const fn code(self) -> &'static [u8] {
        match self {
            Color::Okay => GREEN,
            Color::Warning => YELLOW,
            Color::Error => RED,
            Color::Transition => YELLOW,
            Color::Service => CYAN,
            Color::Path => GREEN,
            Color::Dim => LIGHT_GRAY,
            Color::Glue => DARK_GRAY,
            Color::NotFound => RED,
            Color::TimeDay => GRAY_250,
            Color::TimeHour => GRAY_248,
            Color::TimeMinute => GRAY_246,
            Color::TimeSecond => GRAY_244,
        }
    }
}

// Cached colorization state
// 0 = uninitialized
// 1 = no colors
// 2 = colors enabled (TTY)
const COLOR_UNINITIALIZED: u8 = 0;
const COLOR_DISABLED: u8 = 1;
const COLOR_ENABLED: u8 = 2;

static SHOULD_COLORIZE: AtomicU8 = AtomicU8::new(COLOR_UNINITIALIZED);

/// Check if colors should be used in output
///
/// Returns true if STDOUT is a TTY.
/// Result is cached after init_colorize() is called.
fn should_colorize() -> bool {
    match SHOULD_COLORIZE.load(Ordering::Relaxed) {
        COLOR_ENABLED => true,
        COLOR_DISABLED => false,
        _ => {
            let is_tty = STDOUT.isatty();
            SHOULD_COLORIZE.store(
                if is_tty {
                    COLOR_ENABLED
                } else {
                    COLOR_DISABLED
                },
                Ordering::Relaxed,
            );
            is_tty
        }
    }
}

pub fn print<T: Print>(s: T) {
    s.print(STDOUT);
}

pub fn println<T: Print>(s: T) {
    s.print(STDOUT);
    b"\n".print(STDOUT);
}

pub fn eprint<T: Print>(s: T) {
    s.print(STDERR);
}

pub fn eprintln<T: Print>(s: T) {
    s.print(STDERR);
    b"\n".print(STDERR);
}

pub fn print_color<T: Print>(color: Color, s: T) {
    if should_colorize() {
        color.print(STDOUT);
        s.print(STDOUT);
        RESET.print(STDOUT);
    } else {
        s.print(STDOUT);
    }
}

pub trait Print {
    fn print(&self, fd: Fd);
    fn print_len(&self) -> usize;

    fn print_padding(&self, width: usize) {
        let len = self.print_len();
        if width > len {
            let mut remaining = width - len;
            while remaining >= 5 {
                b"     ".print(STDOUT);
                remaining -= 5;
            }
            while remaining > 0 {
                b" ".print(STDOUT);
                remaining -= 1;
            }
        }
    }
}

impl Print for Color {
    fn print(&self, fd: Fd) {
        let _ = fd.write(self.code());
    }

    fn print_len(&self) -> usize {
        0
    }
}

impl Print for &[u8] {
    fn print(&self, fd: Fd) {
        let _ = fd.write(self);
    }

    fn print_len(&self) -> usize {
        self.len()
    }
}

impl<const N: usize> Print for [u8; N] {
    fn print(&self, fd: Fd) {
        let _ = fd.write(self);
    }

    fn print_len(&self) -> usize {
        N
    }
}

impl Print for &str {
    fn print(&self, fd: Fd) {
        let _ = fd.write(self.as_bytes());
    }

    fn print_len(&self) -> usize {
        self.len()
    }
}

impl Print for &CStr {
    fn print(&self, fd: Fd) {
        let _ = fd.write(self.to_bytes());
    }

    fn print_len(&self) -> usize {
        self.to_bytes().len()
    }
}

impl Print for u32 {
    fn print(&self, fd: Fd) {
        let _ = fd.write(itoa::Buffer::new().format(*self).as_bytes());
    }

    fn print_len(&self) -> usize {
        itoa::Buffer::new().format(*self).len()
    }
}

impl Print for u64 {
    fn print(&self, fd: Fd) {
        let _ = fd.write(itoa::Buffer::new().format(*self).as_bytes());
    }

    fn print_len(&self) -> usize {
        itoa::Buffer::new().format(*self).len()
    }
}

impl Print for pid_t {
    fn print(&self, fd: Fd) {
        let _ = fd.write(itoa::Buffer::new().format(*self).as_bytes());
    }

    fn print_len(&self) -> usize {
        itoa::Buffer::new().format(*self).len()
    }
}

impl Print for usize {
    fn print(&self, fd: Fd) {
        let _ = fd.write(itoa::Buffer::new().format(*self).as_bytes());
    }

    fn print_len(&self) -> usize {
        itoa::Buffer::new().format(*self).len()
    }
}
