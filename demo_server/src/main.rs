extern crate bytes;
extern crate bincode;
extern crate core;
extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate serde;
#[macro_use]extern crate serde_derive;
extern crate uuid;
extern crate spatiub;
extern crate tokio;

mod entity;
mod network;

use std::error::Error;
use log::LevelFilter;
use spatiub::spatial::SpatialChannel;
use spatiub::spatial::MapDefinition;
use spatiub::spatial::Entity;
use spatiub::futures_sub;
use entity::DemoEntity;
use uuid::Uuid;
use spatiub::spatial::Point;
use spatiub::spatial::SpatialEvent;

fn main() -> Result<(), Box<Error>> {
    // Always print backtrace on panic.
    ::std::env::set_var("RUST_BACKTRACE", "1");

    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .filter_level(LevelFilter::Info)
        .init();

    info!("Hello, world!");

    let mut channel = SpatialChannel::new(MapDefinition{
        zone_width: 16,
        map_width_in_zones: 1000,
    });

    let entity = DemoEntity{
        id: Uuid::new_v4(),
    };

    let position = Point(0, 0);

    let (subscriber, _receiver) = futures_sub::new_subscriber(entity.id().clone());

    channel.subscribe(subscriber, &position);

    let destination = Point(1, 0);
    channel.publish(SpatialEvent{
        to: Some(destination),
        from: position,
        acting_entity: entity,
        is_a_move: true,
    });

    Ok(())
}