// #![warn(missing_docs)]

#[macro_use]
extern crate log;

// #[macro_use]
// extern crate enum_primitive;

pub mod wm;
pub mod ipc;

mod bindings;
mod xconnection;
mod client;
mod events;

pub use xconnection::XcbConnection;
pub use wm::WindowManager;
