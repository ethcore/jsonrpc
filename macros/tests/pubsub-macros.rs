extern crate jsonrpc_core;
extern crate jsonrpc_pubsub;
#[macro_use]
extern crate jsonrpc_macros;

use std::sync::Arc;
use jsonrpc_core::futures::{future, BoxFuture, Future};
use jsonrpc_core::futures::sync::mpsc;
use jsonrpc_core::Error;
use jsonrpc_pubsub::{PubSubHandler, SubscriptionId, Session, PubSubMetadata};
use jsonrpc_macros::{pubsub, Trailing};

build_rpc_trait! {
	pub trait Rpc {
		type Metadata;

		#[pubsub(name = "hello")] {
			/// Hello subscription
			#[rpc(name = "hello_subscribe")]
			fn subscribe(&self, Self::Metadata, pubsub::Subscriber<String>, u32, Trailing<u64>);

			/// Unsubscribe from hello subscription.
			#[rpc(name = "hello_unsubscribe")]
			fn unsubscribe(&self, SubscriptionId) -> BoxFuture<bool, Error>;
		}
	}
}

#[derive(Default)]
struct RpcImpl;

impl Rpc for RpcImpl {
	type Metadata = Metadata;

	fn subscribe(&self, _meta: Self::Metadata, subscriber: pubsub::Subscriber<String>, _pre: u32, _trailing: Trailing<u64>) {
		let _sink = subscriber.assign_id(SubscriptionId::Number(5));
	}

	fn unsubscribe(&self, _id: SubscriptionId) -> BoxFuture<bool, Error> {
		future::ok(true).boxed()
	}
}

#[derive(Clone, Default)]
struct Metadata;
impl jsonrpc_core::Metadata for Metadata {}
impl PubSubMetadata for Metadata {
	fn session(&self) -> Option<Arc<Session>> {
		let (tx, _rx) = mpsc::channel(1);
		Some(Arc::new(Session::new(tx)))
	}
}

#[test]
fn test_invalid_trailing_pubsub_params() {
	let mut io = PubSubHandler::default();
	let rpc = RpcImpl::default();
	io.extend_with(rpc.to_delegate());

	// when
	let meta = Metadata;
	let req = r#"{"jsonrpc":"2.0","id":1,"method":"hello_subscribe","params":[]}"#;
	let _res = io.handle_request_sync(req, meta);
}
