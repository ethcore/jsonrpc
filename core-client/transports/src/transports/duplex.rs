//! Duplex transport

use failure::format_err;
use futures::prelude::*;
use futures::sync::{mpsc, oneshot};
use jsonrpc_core::Id;
use log::debug;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::VecDeque;

use super::RequestBuilder;
use crate::{RpcChannel, RpcError, RpcMessage};

/// The Duplex handles sending and receiving asynchronous
/// messages through an underlying transport.
pub struct Duplex<TSink, TStream> {
	request_builder: RequestBuilder,
	queue: HashMap<Id, oneshot::Sender<Result<Value, RpcError>>>,
	sink: TSink,
	stream: TStream,
	channel: Option<mpsc::Receiver<RpcMessage>>,
	outgoing: VecDeque<String>,
}

impl<TSink, TStream> Duplex<TSink, TStream> {
	/// Creates a new `Duplex`.
	fn new(sink: TSink, stream: TStream, channel: mpsc::Receiver<RpcMessage>) -> Self {
		Duplex {
			request_builder: RequestBuilder::new(),
			queue: HashMap::new(),
			sink,
			stream,
			channel: Some(channel),
			outgoing: VecDeque::new(),
		}
	}
}

/// Creates a new `Duplex`, along with a channel to communicate
pub fn duplex<TSink, TStream>(sink: TSink, stream: TStream) -> (Duplex<TSink, TStream>, RpcChannel) {
	let (sender, receiver) = mpsc::channel(0);
	let client = Duplex::new(sink, stream, receiver);
	(client, sender.into())
}

impl<TSink, TStream> Future for Duplex<TSink, TStream>
where
	TSink: Sink<SinkItem = String, SinkError = RpcError>,
	TStream: Stream<Item = String, Error = RpcError>,
{
	type Item = ();
	type Error = RpcError;

	fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
		// Handle requests from the client.
		loop {
			if self.channel.is_none() {
				break;
			}
			let msg = match self.channel.as_mut().expect("channel is some; qed").poll() {
				Ok(Async::Ready(Some(msg))) => msg,
				Ok(Async::Ready(None)) => {
					// When the channel is dropped we still need to finish
					// outstanding requests.
					self.channel.take();
					break;
				}
				Ok(Async::NotReady) => break,
				Err(()) => continue,
			};
			let (id, request_str) = self.request_builder.single_request(&msg);
			self.queue.insert(id, msg.sender);
			self.outgoing.push_back(request_str);
		}
		// Handle outgoing rpc requests.
		loop {
			match self.outgoing.pop_front() {
				Some(request) => match self.sink.start_send(request)? {
					AsyncSink::Ready => {}
					AsyncSink::NotReady(request) => {
						self.outgoing.push_front(request);
						break;
					}
				},
				None => break,
			}
		}
		let done_sending = match self.sink.poll_complete()? {
			Async::Ready(()) => true,
			Async::NotReady => false,
		};
		// Handle incoming rpc requests.
		loop {
			let response_str = match self.stream.poll() {
				Ok(Async::Ready(Some(response_str))) => response_str,
				Ok(Async::Ready(None)) => {
					// The websocket connection was closed so the client
					// can be shutdown. Reopening closed connections must
					// be handled by the transport.
					debug!("connection closed");
					return Ok(Async::Ready(()));
				}
				Ok(Async::NotReady) => break,
				Err(err) => Err(err)?,
			};

			let responses = super::parse_response(&response_str)?;
			for (id, result) in responses {
				let channel = self.queue.remove(&id);
				match channel {
					Some(tx) => tx
						.send(result)
						.map_err(|_| RpcError::Other(format_err!("oneshot channel closed")))?,
					None => Err(RpcError::UnknownId)?,
				};
			}
		}
		if self.channel.is_none() && self.outgoing.is_empty() && self.queue.is_empty() && done_sending {
			debug!("client finished");
			Ok(Async::Ready(()))
		} else {
			Ok(Async::NotReady)
		}
	}
}
