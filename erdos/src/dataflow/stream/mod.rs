//! Streams are used to send data between [operators](crate::dataflow::operator).
//!
//! In the driver, connections between operators are created by passing
//! [`Stream`]s as arguments to the [connect functions](crate::dataflow::connect).
//!
//! During execution, operators can broadcast data to all downstream operators
//! connected to a stream by invoking [`WriteStreamT::send`].
//! Likewise, operators can process data by implementing callbacks
//! in the [operator traits](crate::dataflow::operator),
//! or by calling [`ReadStream::read`] or [`ReadStream::try_read`] in an
//! operator's `run` method.
//!
//! The driver can interact with an application by sending messages on an
//! [`IngestStream`] or reading messages from an [`ExtractStream`].
//!
//! Messages sent on a stream are broadcast to all connected operators,
//! using zero-copy communication for operators on the same node.
//! Messages sent across nodes are serialized using
//! [abomonation](https://github.com/TimelyDataflow/abomonation) if possible,
//! before falling back to [bincode](https://github.com/servo/bincode).
use std::marker::PhantomData;

use crate::dataflow::{Data, Message};

// Private submodules
mod extract_stream;
mod ingest_stream;
mod loop_stream;
mod read_stream;
mod write_stream;

// Public submodules
pub mod errors;

// Private imports
use errors::SendError;

// Public exports
pub use extract_stream::ExtractStream;
pub use ingest_stream::IngestStream;
#[doc(hidden)]
pub use loop_stream::LoopStream;
pub use read_stream::ReadStream;
pub use write_stream::WriteStream;

use super::graph::default_graph;

pub type StreamId = crate::Uuid;

/// Write stream trait which allows specialized implementations of
/// [`send`](WriteStreamT::send) depending on the serialization library used.
pub trait WriteStreamT<D: Data> {
    /// Sends a messsage to a channel.
    fn send(&mut self, msg: Message<D>) -> Result<(), SendError>;
}

pub trait Stream<D: Data> {
    fn name(&self) -> String {
        default_graph::get_stream_name(&self.id())
    }
    fn set_name(&mut self, name: &str) {
        default_graph::set_stream_name(&self.id(), name);
    }
    fn id(&self) -> StreamId;
}

#[derive(Clone)]
pub struct OperatorStream<D: Data> {
    /// The unique ID of the stream (automatically generated by the constructor)
    id: StreamId,
    phantom: PhantomData<D>,
}

#[allow(dead_code)]
impl<D: Data> OperatorStream<D> {
    /// Creates a new stream.
    pub(crate) fn new() -> Self {
        let id = StreamId::new_deterministic();

        Self {
            id,
            phantom: PhantomData,
        }
    }
}

impl<D: Data> Stream<D> for OperatorStream<D> {
    fn id(&self) -> StreamId {
        self.id
    }
}
