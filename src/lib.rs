#![cfg(windows)]

mod serial;
mod hook;
mod config;
mod utils;
pub use hook::*;
pub use serial::*;

pub fn main() {
    serial::main(); // https://github.com/rust-lang/rust/issues/28937
}