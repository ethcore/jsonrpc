extern crate serde;
extern crate jsonrpc_core;
#[macro_use]
extern crate jsonrpc_derive;

use jsonrpc_core::Result;

#[rpc]
pub trait Rpc {
	/// Returns a protocol version
	#[rpc(Xname = "protocolVersion")]
	fn protocol_version(&self) -> Result<String>;
}

fn main() {}
