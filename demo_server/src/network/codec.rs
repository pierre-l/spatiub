use std::io;
use std::io::Error;
use std::io::ErrorKind;
use bytes::{BufMut, BytesMut, BigEndian, ByteOrder};
use tokio::codec::Decoder;
use tokio::codec::Encoder;
use bincode;
use std::marker::PhantomData;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub struct LengthFieldBasedCodec<M, C>
    where C: Encoder<Item=M, Error=Error> + Decoder<Item=M, Error=Error>
{
    inner: C,
    phantom: PhantomData<M>
}

impl <'de, C, M> Decoder for LengthFieldBasedCodec<M, C>
    where
        C: Encoder<Item=M, Error=Error> + Decoder<Item=M, Error=Error>,
        M: DeserializeOwned
{
    type Item = <C as Decoder>::Item;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<M>, io::Error> {
        // At least 4 bytes are required for a frame: 4 byte length field + the payload
        if buf.len() < 4 {
            return Ok(None);
        }

        let length_field = BigEndian::read_u32(&buf.as_ref()[0..4]);

        if buf.len() < length_field as usize {
            return Ok(None);
        }

        buf.split_to(4); // The frame is whole, strip the length field.
        let line = buf.split_to(length_field as usize);

        match bincode::deserialize(line.as_ref()) {
            Ok(deserialized) => {
                Ok(Some(deserialized))
            },
            Err(err) => {
                Err(Error::new(ErrorKind::Other, err))
            }
        }
    }
}

impl <C, M> Encoder for LengthFieldBasedCodec<M, C>
    where
        C: Encoder<Item=M, Error=Error> + Decoder<Item=M, Error=Error>,
        M: Serialize,
{
    type Item = M;
    type Error = Error;

    fn encode(&mut self, msg: M, buf: &mut BytesMut) -> io::Result<()> {
        match bincode::serialize(&msg) {
            Ok(serialized) => {
                let len = serialized.len();
                buf.reserve(len + 4);
                buf.put_u32_be(len as u32);

                buf.extend(&serialized);

                Ok(())
            },
            Err(err) => {
                Err(Error::new(ErrorKind::Other, err))
            }
        }
    }
}
