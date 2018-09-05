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
use spatiub::spatial::Entity;
use spatiub::spatial::Point;
use spatiub::spatial::SpatialEvent;
use std::net::SocketAddr;
use std::ops::Add;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use tokio::runtime::current_thread::Runtime;
use tokio::timer::Delay;
use std::cell::RefCell;
use spatiub::spatial::MapDefinition;
use rand::ThreadRng;
use rand::thread_rng;

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

    let map = MapDefinition::new(16, 16*16);

    let addr: SocketAddr = "127.0.0.1:6142".parse().unwrap();
    let addr_clone = addr.clone();
    let map_clone = map.clone();
    thread::spawn(move ||{
        server::server(&addr_clone, map_clone);
        info!("Server stopped");
    });

    thread::sleep(Duration::from_millis(100));

    let client_entity_id = RefCell::new(None);
    let client_future = client::client(
        &addr,
        |message|{
        if let Message::ConnectionAck(entity) = &message {
            client_entity_id.replace(Some(entity.id().clone()));
        } else if let Message::Event(event) = &message {
            let involves_client_entity = if let Some(ref client_entity_id) = *client_entity_id.borrow(){
                event.acting_entity.id() == client_entity_id
            } else {
                false
            };

            if involves_client_entity{
                if let Some(ref destination) = &event.to {
                    let latency = event.acting_entity.last_state_update.elapsed();

                    let latency = latency.subsec_nanos();

                    info!("Position: {:?}, Latency: {}", destination, latency);
                }
            }
        }

        // PERFORMANCE Suboptimal. No need to send a delay future if result == None.
        // PERFORMANCE Suboptimal. Is there a way to avoid calling thread_rng everytime?
        delay(message, &map, &mut thread_rng())
    });

    let mut runtime = Runtime::new().unwrap();
    if let Err(_err) = runtime.block_on(client_future) {
        info!("Client stopped");
    }
}

fn trigger_new_move(mut entity: DemoEntity, from: Point, to: Point) -> Option<Message> {
    entity.last_state_update = Timestamp::new();
    let event = Message::Event(SpatialEvent {
        from,
        to: Some(to),
        acting_entity: entity,
        is_a_move: true,
    });

    Some(event)
}

fn delay(message: Message, map: &MapDefinition, rng: &mut ThreadRng) -> impl Future<Item=Option<Message>, Error=()> {
    // TODO Determine next position.
    let next_position = if let Message::Event(ref event) = &message {
        if let Some(ref destination) = &event.to{
            Some(map.random_point_next_to(destination, rng))
        } else {
            None
        }
    } else {
        None
    };

    Delay::new(Instant::now().add(Duration::from_millis(500)))
        .map(move |()| {
            if let Message::Event(event) = message {
                if let Some(event_destination) = event.to {
                    trigger_new_move(event.acting_entity, event_destination, next_position.unwrap())
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