use std::io;

use actix::Message;
use bytes::BytesMut;
use log::info;
use tokio::codec::{Decoder, Encoder};

/// Message coming from the network
#[derive(Debug, Message)]
pub enum Request {
    /// Request message
    Message(String),
}

/// Message going to the network
#[derive(Debug, Message)]
pub enum Response {
    /// Response message
    Message(String),
}

/// Codec for client -> server transport
pub struct P2PCodec;

/// Implement decoder trait for P2P codec
impl Decoder for P2PCodec {
    type Item = Request;
    type Error = io::Error;

    /// Method to decode bytes to a request
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Locate a byte corresponding to a '\n' in the byte stream
        if let Some(i) = src.iter().position(|&b| b == b'\n') {
            // Remove the serialized frame from the buffer.
            let line = src.split_to(i + 1);

            // Parse the buffer as an UTF-8 encoded string
            let mut res = String::from_utf8(line.to_vec()).unwrap();

            // Remove the last two bytes of the string (corresponding to \r\n)
            res.truncate(res.len() - 2);

            Ok(Some(Request::Message(res)))
        } else {
            Ok(None)
        }
    }
}

/// Implement encoder trait for P2P codec
impl Encoder for P2PCodec {
    type Item = Response;
    type Error = io::Error;

    /// Method to encode a response into bytes
    fn encode(&mut self, msg: Response, _dst: &mut BytesMut) -> Result<(), Self::Error> {
        info!("Encoding {:?}", msg);

        // TODO: Encoder to be completed

        Ok(())
    }
}
