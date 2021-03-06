//! JSON-RPC client implementation primitives.
//!
//! By default this crate does not implement any transports,
//! use corresponding features (`tls`, `http` or `ws`) to opt-in for them.
//!
//! See documentation of [`jsonrpc-client-transports`](https://docs.rs/jsonrpc-client-transports) for more details.

#![deny(missing_docs)]

pub use futures;
pub use jsonrpc_client_transports::*;
