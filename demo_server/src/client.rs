use futures::{Future, Stream, Sink};
use server::codec;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::runtime::current_thread::Runtime;
use tokio_codec::Decoder;
use message::Message;

pub fn run_client<F>(addr: &SocketAddr, message_consumer: F)
    where F: Fn(Message) -> Result<Option<Message>, ()>
{
    let client_future = TcpStream::connect(&addr)
        .map_err(|err|{
            panic!("Connection failed. Cause: {}", err)
        })
        .and_then(move |socket| {
            info!("Connection established");
            let (output, input) = codec().framed(socket).split();
            let output = output.sink_map_err(|err| error!("An error occurred in the input stream: {}", err));

            input
                .map_err(|err| error!("An error occurred in the input stream: {}", err))
                .then(move |result|{
                    match result {
                        Ok(message) => {
                            message_consumer(message)
                        }
                        Err(err) => Err(err),
                    }
                })
                .filter_map(|message| message)
                .forward(output)
        });

    let mut runtime = Runtime::new().unwrap();
    if let Err(_err) = runtime.block_on(client_future) {
        info!("Client stopped");
    }
}