// Internal configuration data generated from user-created configuration data.
//
// This resolves to:
//
//     target/{debug,release}/build/connate-*/out/config.rs
//
// If you see compiler errors about OUT_DIR being unset, it may be because this file is incorrectly
// called recursively.  Ensure src/build/ does not `mod` this, and that instead other binaries
// which need it `mod` it directly.
include!(concat!(env!("OUT_DIR"), "/config.rs"));
