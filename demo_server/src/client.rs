use futures::{Future, Sink, Stream};
use message::Message;
use server::codec;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio_codec::Decoder;

pub fn client<C, F>(addr: &SocketAddr, message_consumer: C)
                 -> impl Future<Item=(), Error=()>
    where
        C: Fn(Message) -> F,
        F: Future<Item=Option<Message>, Error=()>,
{
    TcpStream::connect(&addr)
        .map_err(|err|{
            panic!("Connection failed. Cause: {}", err)
        })
        .and_then(move |socket| {
            info!("Connection established");
            let (output, input) = codec().framed(socket).split();
            let output = output.sink_map_err(|err| error!("An error occurred in the input stream: {}", err));

            input
                .map_err(|err| error!("An error occurred in the input stream: {}", err))
                .map(move |message| {
                    message_consumer(message)
                })
                .buffered(100000)
                .filter_map(|message| message)
                .forward(output)
        })
        .map(|_|{})
}