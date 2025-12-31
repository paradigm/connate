#![cfg_attr(not(test), allow(unused_attributes))]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

// Implementation of features Rust core expects from libc
//
// This conflicts with anything which uses std, including src/build/main.rs and tests.  To avoid
// this conflict, connate and conctl `mod` this directly rather than `mod`ing it in the shared
// `lib.rs`.
//
// mod libc_shim;

// Internal representations of user-facing config.rs
//
// We don't want the generated file to be loaded by src/build/main.rs lest it cyclically
// depend on itself.  Thus, this file should not be `mod`'d by either build or lib.rs.  Instead,
// connate and conctl `mod` it directly.
//
// mod internal;

pub mod config;
pub mod constants;
pub mod err;
pub mod internal_api;
pub mod ipc;
pub mod os;
pub mod syscall;
pub mod types;
pub mod util;
