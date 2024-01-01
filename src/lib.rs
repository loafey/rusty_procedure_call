#![doc = include_str!("../readme.md")]
pub use error::RpcError;
pub use rpc_derive::rpc;
extern crate self as rusty_procedure_call;

pub mod postcard {
    pub use postcard::*;
}

mod error;
#[cfg(test)]
mod tests;
