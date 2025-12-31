#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

// Implementation of features Rust core expects from libc
//
// This conflicts with anything which uses std, including src/build/main.rs and tests.  To avoid
// this conflict, connate and conctl `mod` this directly rather than `mod`ing it in the shared
// `lib.rs`.
//
// We don't need to use this, just make it visible to the linker.
#[cfg(not(test))]
#[path = "../libc_shim/mod.rs"]
mod libc_shim;

// Internal representations of user-facing config.rs
//
// We don't want the generated file to be loaded by src/build/main.rs lest it cyclically
// depend on itself.  Thus, this file should not be `mod`'d by either build or lib.rs.  Instead,
// connate and conctl `mod` it directly.
#[allow(unused)]
#[path = "../internal.rs"]
mod internal;

mod cmd;

/// # Safety
///
/// Platform ABI guarantees incoming C-style format
#[cfg(not(test))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(
    argc: isize,
    argv: *const *const core::ffi::c_char,
    envp: *const *const core::ffi::c_char,
) -> isize {
    let argv = unsafe { connate::os::Argv::from_raw(argc, argv) };
    let envp = unsafe { connate::os::Envp::from_raw(envp) };

    cmd::Cmd::new(argv, envp, internal::CONFIG_LOCK_FILE).run()
}
