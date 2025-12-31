// Interface that config should fulfill.
//
// User should read through this to understand how to configure Connate.
mod config_api;
pub use config_api::*;

// User-defined configuration.
#[allow(clippy::module_inception)]
mod config;

// Helper functions for the user's config.rs implementation.
#[allow(unused)]
mod helpers;
