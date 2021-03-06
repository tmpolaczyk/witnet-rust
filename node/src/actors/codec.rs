use std::convert::TryFrom;
use std::io;
use std::io::Cursor;

use byteorder::{BigEndian, ReadBytesExt};

use actix::Message;
use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

const HEADER_SIZE: usize = 4; // bytes

/// Codec for client -> server transport
///
/// Format:
/// ```ignore
/// Message size: u32
/// Message: [u8; Message size]
/// ```
///
/// The message format is described in the file [schemas/protocol.fbs][protocol]
///
/// [protocol]: https://github.com/witnet/witnet-rust/blob/master/schemas/protocol.fbs
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct P2PCodec;

impl Message for P2PCodec {
    type Result = ();
}

/// Implement decoder trait for P2P codec
impl Decoder for P2PCodec {
    type Item = BytesMut;
    type Error = io::Error;

    /// Method to decode bytes to a request
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut ftb: Option<Self::Item> = None;
        let msg_len = src.len();
        if msg_len >= HEADER_SIZE {
            let mut header_vec = Cursor::new(&src[0..HEADER_SIZE]);
            let msg_size = usize::try_from(header_vec.read_u32::<BigEndian>().unwrap()).unwrap();
            if msg_len - HEADER_SIZE >= msg_size {
                src.advance(HEADER_SIZE);
                ftb = Some(src.split_to(msg_size));
            }
        }
        // If the message is incomplete, return without consuming anything.
        // This method will be called again when more bytes arrive.

        Ok(ftb)
    }
}

/// Implement encoder trait for P2P codec
impl Encoder<BytesMut> for P2PCodec {
    type Error = io::Error;

    /// Method to encode a response into bytes
    fn encode(&mut self, bytes: BytesMut, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let header: u32 = u32::try_from(bytes.len()).map_err(|_| {
            log::error!("Maximum message size exceeded");

            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Message size {} bytes too big for u32", bytes.len()),
            )
        })?;
        // push header with msg len
        dst.put_u32(header);
        // push message
        dst.put(bytes);
        Ok(())
    }
}
