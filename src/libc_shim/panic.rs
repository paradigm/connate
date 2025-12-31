use connate::os::{Print, eprint, eprintln, exit};

/// Panic handler
///
/// The Connate project strives to ensure the compiler can prove no Connate code could panic.
/// However:
/// - Some part of Rust (rustc? core?) expects a panic handler, even if it is removed as dead code
/// - User-defined configuration may panic in forked-off but not exec'd processes which continue to
///   use the main connate binary, and so a panic handler is necessary for user code
#[cfg_attr(not(test), panic_handler)]
fn panic(info: &core::panic::PanicInfo) -> ! {
    #[cfg(debug_assertions)]
    {
        eprint("Panic!");
        if let Some(e) = info.message().as_str() {
            eprint(" ");
            eprint(e);
        }
        eprint("\n");
        if let Some(loc) = info.location() {
            eprint("File: ");
            eprint(loc.file());
            eprint(":");
            eprint(loc.line());
            eprint(":");
            eprint(loc.column());
            eprint("\n");
        }
    }

    // If this code path shows up in the resulting binary, it means the compiler thinks a panic is
    // possible.
    //
    // Build tooling should run
    // ```
    // ./check_panic.sh
    // ```
    // to check if the compiler has failed to prove to itself that panics are impossible
    //
    // This should be done with a minimal config.rs, as user config.rs are allowed to include
    // panics in forked-off processes that will not bring down the main process.
    eprintln("unexpected panic");
    exit(1);
}
