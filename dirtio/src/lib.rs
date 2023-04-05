#![allow(clippy::module_inception)]

mod io;
pub mod net;
pub mod runtime;

pub use dirtio_macros::main;
pub use runtime::scheduler::spawn;
