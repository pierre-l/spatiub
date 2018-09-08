use futures::{Future, Sink, Stream};
use message::Message;
use server::codec;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio_codec::Decoder;

pub fn client<C, F>(addr: &SocketAddr, message_consumer: C)
                 -> impl Future<Item=(), Error=()>
    where
        C: Fn(Message) -> Option<F>,
        F: Future<Item=Message, Error=()>,
{
    TcpStream::connect(&addr)
        .map_err(|err|{
            panic!("Connection failed. Cause: {}", err)
        })
        .and_then(move |socket| {
            debug!("Connection established");
            let (output, input) = codec().framed(socket).split();
            let output = output.sink_map_err(|err| error!("An error occurred in the input stream: {}", err));

            input
                .map_err(|err| error!("An error occurred in the input stream: {}", err))
                .map(move |message| {
                    message_consumer(message)
                })
                .filter_map(|future| future)
                .buffered(100000)
                .forward(output)
        })
        .map(|_|{})
}