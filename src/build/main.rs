// build.rs has its own module namespace, separate from the main crate.
//
// However, since we're utilizing the user-created config.rs here, we need to share the same module
// namespace as the user would expect to see when configuring the system so that `mod` or `use`
// lines in `config.rs` are valid in this context as well.
//
// Thus, we're manually loading lib.rs and bringing its contents directly into our module
// namespace
#[path = "../lib.rs"]
mod connate;
pub use connate::*;

pub mod build_util;
use crate::config::Connate;

mod check;
use check::ConfigCheck;
impl ConfigCheck for Connate {}

mod generate;
use generate::GenerateInternalConfig;
impl GenerateInternalConfig for Connate {}

fn main() {
    Connate::check_config();

    let out_dir = std::env::var("OUT_DIR").expect("Expected OUT_DIR to be set");
    let path = std::path::Path::new(&out_dir).join("config.rs");
    let f = std::fs::File::create(&path).expect("Failed to create output file");

    Connate::generate_internal_config(f).expect("Expected to be able to write to {path}");

    // Keep last so that these messages do not show up in user-facing config.rs check errors.
    compiler_instructions();
}

/// Linker arguments needed for connate and conctl to be nostd, nolibc programs.
fn compiler_instructions() {
    for bin in ["connate", "conctl"] {
        println!("cargo:rustc-link-arg-bin={bin}=-nostartfiles");
        println!("cargo:rustc-link-arg-bin={bin}=-nostdlib");
        println!("cargo:rustc-link-arg-bin={bin}=-static");
        println!("cargo:rustc-link-arg-bin={bin}=-no-pie");
    }
}
