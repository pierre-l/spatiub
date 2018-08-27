#[macro_use] extern crate criterion;
extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate uuid;
extern crate spatiub;

use criterion::Criterion;
use uuid::Uuid;
use futures::future;
use futures::Stream;
use futures::Future;
use spatiub::spatial::SpatialChannel;
use spatiub::spatial::MapDefinition;
use spatiub::futures_sub;
use spatiub::spatial::Point;
use spatiub::spatial::SpatialEvent;
use spatiub::spatial::Entity;
use log::LevelFilter;

const ZONE_WIDTH: usize = 16;
fn bench_sending(c: &mut Criterion) {
    c.bench_function("bench_sending", |b| {
        let map_width_in_zones = 1000;
        let map_width = map_width_in_zones * ZONE_WIDTH;
        let mut channel = SpatialChannel::new(
            MapDefinition {
                zone_width: ZONE_WIDTH,
                map_width_in_zones,
            }
        );

        let entity_id = Uuid::new_v4();

        b.iter(|| {
            let (subscriber, mut receiver) = futures_sub::new_subscriber(entity_id);

            let number_of_events = 1000;
            let mut position = Point(0, 0);
            channel.subscribe(subscriber, &position);

            for _i in 0..number_of_events {
                let new_x = position.0 + 1;

                if new_x >= map_width {
                    panic!()
                }

                let destination = Point(new_x, position.1);

                channel.publish(SpatialEvent{
                    from: position,
                    to: Some(destination.clone()),
                    acting_entity: TestEntity{
                        id: entity_id,
                    },
                    is_a_move: true,
                });
                position = destination;
            }


            let mut number_of_events_left = number_of_events;

            let stream = receiver
                .map_err(|err|{
                    EndOfStream::Unexpected
                })
                .for_each(|event|{
                number_of_events_left -= 1;
                if number_of_events_left > 1 {
                    future::ok(())
                } else {
                    future::err(EndOfStream::OutOfEvents)
                }
            });

            assert_eq!(EndOfStream::OutOfEvents, stream.wait().err().unwrap());
        });

        drop(channel);
    });
}

#[derive(Debug, PartialEq)]
enum EndOfStream{
    OutOfEvents,
    Unexpected,
}

#[derive(Clone)]
struct TestEntity{
    id: Uuid,
}

impl Entity for TestEntity{
    fn id(&self) -> &Uuid {
        &self.id
    }
}

criterion_group!(benches, bench_sending);
criterion_main!(benches);