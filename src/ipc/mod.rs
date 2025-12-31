//! Inter-process communication between connate, conctl, and daemon supervisor

mod ipc_client;
mod ipc_server;
mod request;
mod response;
pub use ipc_client::*;
pub use ipc_server::*;
pub use request::*;
pub use response::*;
