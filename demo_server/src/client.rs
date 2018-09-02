use futures::{Future, Stream, Sink};
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
            let output = output.sink_map_err(|err| error!("An error occurred in the input stream: {}", err));

            input
                .map_err(|err| error!("An error occurred in the input stream: {}", err))
                .then(move |result|{
                    match result {
                        Ok(message) => {
                            info!("Message received: {:?}", message);

                            match message {
                                Message::ConnectionAck(entity) => {
                                    let event = Message::Event(SpatialEvent {
                                        from: Point(0, 0),
                                        to: Some(Point(1, 0)),
                                        acting_entity: entity,
                                        is_a_move: true,
                                    });
                                    Ok(Some(event))
                                },
                                Message::Event(event) => {
                                    if let Some(Point(1, 0)) = event.to{
                                        info!("Stopping the client.");
                                        Err(())
                                    } else {
                                        Ok(None)
                                    }
                                },
                            }
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