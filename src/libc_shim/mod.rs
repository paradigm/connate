//! Rust minimal core crate requirements typically provided by libc.
//!
//! These should not be `use`'d by any module in this project, only `mod`'d or `extern crate`'d.
//! The linker will then find them and use them to satisfy `core::` requirements.
//!
//! This is mutually-exclusive with Rust std, which is automatically linked to for unit tests (via
//! lib.rs) or integration tests (in tests/) and thus should be kept out of those files.  Only the
//! nostd, nolibc connate and conctl main binaries should bring this in.

#![allow(unused)]
#![no_std]

mod bcmp;
mod memcpy;
mod panic;
mod startup;
mod strlen;
pub use bcmp::*;
pub use memcpy::*;
pub use startup::*;
pub use strlen::*;
