#![doc = include_str!("../readme.md")]
pub use error::RpcError;
pub use rpc_derive::rpc;

mod error;
#[cfg(test)]
mod tests;
