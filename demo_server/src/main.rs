extern crate bincode;
extern crate bytes;
extern crate core;
extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate serde;
#[macro_use]extern crate serde_derive;
extern crate spatiub;
extern crate tokio;
extern crate tokio_codec;
extern crate uuid;

use log::LevelFilter;
use std::thread;
use std::net::SocketAddr;
use std::time::Duration;
use message::Message;
use spatiub::spatial::SpatialEvent;
use spatiub::spatial::Point;
use tokio::runtime::current_thread::Runtime;
use entity::Timestamp;
use entity::DemoEntity;
use futures::Future;
use tokio::timer::Delay;
use std::time::Instant;
use std::ops::Add;

mod entity;
mod codec;
mod message;
mod server;
mod client;

fn main() {
    // Always print backtrace on panic.
    ::std::env::set_var("RUST_BACKTRACE", "1");

    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .filter_level(LevelFilter::Info)
        .init();


    let addr: SocketAddr = "127.0.0.1:6142".parse().unwrap();
    let addr_clone = addr.clone();
    thread::spawn(move ||{
        server::server(&addr_clone);
        info!("Server stopped");
    });

    thread::sleep(Duration::from_millis(100));

    let client_future = client::client(&addr, move |message|{
        if let Message::Event(event) = &message {
            if event.to.is_some() {
                let latency = event.acting_entity.last_state_update.elapsed();

                let latency = latency.subsec_nanos();

                info!("Latency: {}", latency);
            }
        }

        // PERFORMANCE Suboptimal. No need to send a delay future if result == None.
        delay(message)
    });

    let mut runtime = Runtime::new().unwrap();
    if let Err(_err) = runtime.block_on(client_future) {
        info!("Client stopped");
    }
}

fn trigger_new_move(mut entity: DemoEntity) -> Option<Message> {
    entity.last_state_update = Timestamp::new();
    let event = Message::Event(SpatialEvent {
        from: Point(0, 0),
        to: Some(Point(1, 0)),
        acting_entity: entity,
        is_a_move: true,
    });

    Some(event)
}

fn delay(message: Message) -> impl Future<Item=Option<Message>, Error=()> {
    Delay::new(Instant::now().add(Duration::from_millis(500)))
        .map(move |()| {
            let result = match &message {
                Message::ConnectionAck(entity) => {
                    trigger_new_move(entity.clone())
                },
                Message::Event(event) => {
                    if event.to.is_some() {
                        trigger_new_move(event.acting_entity.clone())
                    } else {
                        None
                    }
                }
            };

            result
        })
        .map_err(|err|{
            panic!("Timer error: {}", err)
        })
}