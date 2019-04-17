use serde::{Serialize, Deserialize};
use serde_json;
use tokio_core::io::{Codec, EasyBuf};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use std::io;
use std::marker::PhantomData;
use std::mem;

pub struct LengthPrefixedJson<In, Out>
    where In: Serialize + Deserialize,
          Out: Serialize + Deserialize
{
    _in: PhantomData<In>,
    _out: PhantomData<Out>,
}

impl<In, Out> LengthPrefixedJson<In, Out>
    where In: Serialize + Deserialize,
          Out: Serialize + Deserialize
{
    pub fn new() -> LengthPrefixedJson<In, Out> {
        LengthPrefixedJson {
            _in: PhantomData,
            _out: PhantomData,
        }
    }
}

// `LengthPrefixedJson` is a codec for sending and receiving serde_json serializable types. The
// over the wire format is a Big Endian u16 indicating the number of bytes in the JSON payload
// (not including the 2 u16 bytes themselves) followed by the JSON payload.
impl<In, Out> Codec for LengthPrefixedJson<In, Out>
    where In: Serialize + Deserialize,
          Out: Serialize + Deserialize
{
    type In = In;
    type Out = Out;

    fn decode(&mut self, buf: &mut EasyBuf) -> io::Result<Option<Self::In>> {
        // Make sure we have at least the 2 u16 bytes we need.
        let msg_size = match buf.as_ref().read_u16::<BigEndian>() {
            Ok(msg_size) => msg_size,
            Err(_) => return Ok(None),
        };
        let hdr_size = mem::size_of_val(&msg_size);
        let msg_size = msg_size as usize + hdr_size;

        // Make sure our buffer has all the bytes indicated by msg_size.
        if buf.len() < msg_size {
            return Ok(None);
        }

        // Drain off the entire message.
        let buf = buf.drain_to(msg_size);

        // Trim off the u16 length bytes.
        let msg_buf = &buf.as_ref()[hdr_size..];

        // Decode!
        let msg: In = serde_json::from_slice(msg_buf)
                   .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        Ok(Some(msg))
    }

    fn encode(&mut self, msg: Out, buf: &mut Vec<u8>) -> io::Result<()> {
        // Encode directly into `buf`.
        serde_json::to_writer(buf, &msg)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        let len = buf.len() as u16;

        // add space for our length
        for _ in 0..mem::size_of_val(&len) {
            buf.insert(0, 0);
        }

        // Insert our length bytes at the front of `buf`.
        let mut cursor = io::Cursor::new(buf);
        cursor.set_position(0);
        cursor.write_u16::<BigEndian>(len)
    }
}
