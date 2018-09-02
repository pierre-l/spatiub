use futures::{future, Future, Stream, Sink};
use futures::unsync::mpsc;
use message::Message;
use server::codec;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::runtime::current_thread::Runtime;
use tokio_codec::Decoder;
use spatiub::spatial::SpatialEvent;
use spatiub::spatial::Point;

pub fn run_client(addr: &SocketAddr,){
    let client_future = TcpStream::connect(&addr)
        .map_err(|err|{
            panic!("Connection failed. Cause: {}", err)
        })
        .and_then(move |socket| {
            info!("Connection established");
            let (output, input) = codec().framed(socket).split();
            let output = output.sink_map_err(|err|{
                error!("An error occurred in the input stream: {}", err)
            });

            let (output_sink, output_stream) = mpsc::unbounded();
            let output = output_stream
                .forward(output);

            input
                .map_err(|err|{
                    error!("An error occurred in the input stream: {}", err)
                })
                .for_each(move |message|{
                info!("Message received: {:?}", message);

                match message {
                    Message::ConnectionAck(entity) => {
                        output_sink.unbounded_send(Message::Event(SpatialEvent{
                            from: Point(0, 0),
                            to: Some(Point(1, 0)),
                            acting_entity: entity,
                            is_a_move: true,
                        })).unwrap();
                        future::ok(())
                    },
                    Message::Event(event) => {
                        if let Some(Point(1, 0)) = event.to{
                            info!("Stopping the client.");
                            future::err(())
                        } else {
                            future::ok(())
                        }
                    }
                }

            })
                .join(output)
        });

    let mut runtime = Runtime::new().unwrap();
    if let Err(_err) = runtime.block_on(client_future) {
        info!("Client stopped");
    }
}