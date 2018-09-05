extern crate bincode;
extern crate bytes;
extern crate core;
extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate rand;
extern crate serde;
#[macro_use]extern crate serde_derive;
extern crate spatiub;
extern crate tokio;
extern crate tokio_codec;
extern crate uuid;

use entity::DemoEntity;
use entity::Timestamp;
use futures::Future;
use log::LevelFilter;
use message::Message;
use spatiub::spatial::Point;
use spatiub::spatial::SpatialEvent;
use std::net::SocketAddr;
use std::ops::Add;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use tokio::runtime::current_thread::Runtime;
use tokio::timer::Delay;

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

    let client_future = client::client(&addr, |message|{
        if let Message::Event(event) = &message {
            if let Some(ref destination) = &event.to {
                let latency = event.acting_entity.last_state_update.elapsed();

                let latency = latency.subsec_nanos();

                info!("Position: {:?}, Latency: {}", destination, latency);
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

fn trigger_new_move(mut entity: DemoEntity, from: Point) -> Option<Message> {
    entity.last_state_update = Timestamp::new();
    let event = Message::Event(SpatialEvent {
        to: Some(Point(from.0 + 1, from.1)),
        from,
        acting_entity: entity,
        is_a_move: true,
    });

    Some(event)
}

fn delay(message: Message) -> impl Future<Item=Option<Message>, Error=()> {
    Delay::new(Instant::now().add(Duration::from_millis(500)))
        .map(move |()| {
            if let Message::Event(event) = message {
                if let Some(destination) = event.to {
                    trigger_new_move(event.acting_entity, destination)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .map_err(|err|{
            panic!("Timer error: {}", err)
        })
}