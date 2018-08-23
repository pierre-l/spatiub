use pub_sub::Subscriber;
use std::collections::HashSet;
use std::rc::Rc;
use uuid::Uuid;
use futures_sub::FutureSubscriber;

pub struct SpatialChannel {
    map_definition: MapDefinition,
    channels: Vec<ZoneChannel>,
}

impl SpatialChannel{
    pub fn new(map_definition: MapDefinition)
        -> SpatialChannel
    {
        let mut channels = vec![];

        for _x in 0..map_definition.map_width_in_zones {
            for _y in 0..map_definition.map_width_in_zones {
                channels.push(ZoneChannel::new());
            }
        }

        SpatialChannel{
            channels,
            map_definition,
        }
    }

    pub fn publish(&mut self, event: SpatialEvent) {
        let event = Rc::new(event);
        let zone_width = self.map_definition.zone_width;

        let mut from_indexes = HashSet::new();
        compute_indexes_for_zones_in_range(&event.from, zone_width, |index|{
            from_indexes.insert(index);
            self.publish_if_channel_exists(index, &event);
        });

        if let Some(ref destination) = event.to {
            compute_indexes_for_zones_in_range(destination, zone_width, |index|{
                if !from_indexes.contains(&index) {
                    self.publish_if_channel_exists(index, &event);
                }
            });
        }
    }

    pub fn subscribe(&mut self, subscriber: FutureSubscriber<SpatialEvent>, position: &Point) {
        let zone_index = zone_index_for_point(position, self.map_definition.zone_width);
        if let Some(channel) = self.channels.get_mut(zone_index) {
            channel.subscribe(subscriber);
        } else {
            panic!()
        }
    }

    fn publish_if_channel_exists(&mut self, channel_index: usize, event: &Rc<SpatialEvent>) {
        let dropped_subscriber_option = if let Some(channel) =  self.channels.get_mut(channel_index) {
            channel.publish(event.clone())
        } else {
            None
        };

        if let Some(ref destination) = event.to {
            if let Some(dropped_entity_subscriber) = dropped_subscriber_option{
                if self.map_definition.point_is_inside(destination) {
                    self.subscribe(dropped_entity_subscriber, destination);
                }
            }
        }
    }
}

pub struct ZoneChannel {
    subscribers: Vec<FutureSubscriber<SpatialEvent>>,
}

impl ZoneChannel where {
    pub fn new() -> ZoneChannel {
        ZoneChannel{
            subscribers: vec![],
        }
    }

    pub fn subscribe(&mut self, subscriber: FutureSubscriber<SpatialEvent>) {
        self.subscribers.push(subscriber);
    }

    pub fn publish(&mut self, event: Rc<SpatialEvent>) -> Option<FutureSubscriber<SpatialEvent>>{
        let mut dropped_subscriber_option= None;

        self.subscribers.retain(|subscriber|{
            match subscriber.send(event.clone()) {
                Ok(retain) => {
                    if event.is_a_move && subscriber.entity_id() == &event.actor_id {
                        dropped_subscriber_option = Some(subscriber.clone());
                        false
                    } else {
                        retain
                    }
                },
                Err(err) => {
                    warn!("Subscriber dropped. Cause: {}", err);
                    false
                }
            }
        });

        dropped_subscriber_option
    }
}

#[derive(Clone)]
pub struct SpatialEvent{
    from: Point,
    to: Option<Point>,
    actor_id: Uuid,
    is_a_move: bool,
}

pub struct MapDefinition{
    zone_width: usize,
    map_width_in_zones: usize
}

impl MapDefinition{
    pub fn point_is_inside(&self, point: &Point) -> bool {
        self.coord_is_inside(&point.0) && self.coord_is_inside(&point.1)
    }

    pub fn coord_is_inside(&self, coord: &usize) -> bool {
        coord < &(&self.zone_width * &self.map_width_in_zones)
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Point(usize, usize);

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Zone(Point, Point);

fn compute_indexes_for_zones_in_range<F>(
    point: &Point,
    zone_width: usize,
    mut consumer: F
) where F: FnMut(usize) {
    let (start_x, x_offset_max) = if point.0 > zone_width {
        (point.0 / zone_width - 1, 3)
    } else {
        (0, 2)
    };

    let (start_y, y_offset_max) = if point.1 > zone_width {
        (point.1 / zone_width - 1, 3)
    } else {
        (0, 2)
    };

    for x_offset in 0..x_offset_max{
        for y_offset in 0..y_offset_max{
            let channel_index = (start_x + x_offset) + (start_y + y_offset) * zone_width;
            consumer(channel_index);
        }
    }
}

fn zone_index_for_point(point: &Point, zone_width: usize) -> usize{
    let x = point.0 / zone_width ;
    let y = point.1 / zone_width;
    x + y * zone_width
}

#[cfg(test)]
mod tests{
    use super::*;
    use futures_sub;
    use futures::{Future, Stream};

    const ZONE_WIDTH: usize = 16;

    #[test]
    pub fn can_subscribe(){
        assert_can_subscribe(&Point(0, 0),
                             event(0, 0, 1, 0));
        assert_can_subscribe(&Point(0, 0),
                             event(ZONE_WIDTH, 0, ZONE_WIDTH+1, 0));
        assert_can_subscribe(&Point(0, 0),
                             event(ZONE_WIDTH, ZONE_WIDTH, ZONE_WIDTH, ZONE_WIDTH+1));
        assert_can_subscribe(&Point(0, 0),
                             event(ZONE_WIDTH, ZONE_WIDTH, ZONE_WIDTH, ZONE_WIDTH+ZONE_WIDTH));
        assert_can_subscribe(&Point(0, 0),
                             event(ZONE_WIDTH+ZONE_WIDTH, ZONE_WIDTH, ZONE_WIDTH, ZONE_WIDTH));
    }

    #[test]
    pub fn subscription_follows_moving_entity() {
        let mut channel = SpatialChannel::new(
            MapDefinition {
                zone_width: ZONE_WIDTH,
                map_width_in_zones: ZONE_WIDTH,
            }
        );

        let entity_id = Uuid::new_v4();
        let (subscriber, mut receiver) = futures_sub::new_subscriber(entity_id);

        let mut position = Point(0, 0);
        channel.subscribe(subscriber, &position);

        for _i in 0..ZONE_WIDTH * 10 {
            let destination = Point(position.0 + 1, position.1);

            channel.publish(SpatialEvent{
                from: position,
                to: Some(destination.clone()),
                actor_id: entity_id,
                is_a_move: true,
            });

            let (received_event_option, receiver_tmp) = receiver.into_future().wait().ok().unwrap();
            receiver = receiver_tmp;

            assert!(received_event_option.is_some());
            position = destination;
        }
    }

    fn assert_can_subscribe(subscription_point: &Point, event: SpatialEvent) {
        let mut channel = SpatialChannel::new(
            MapDefinition {
                zone_width: ZONE_WIDTH,
                map_width_in_zones: ZONE_WIDTH,
            }
        );
        let (subscriber, receiver) = futures_sub::new_subscriber(Uuid::new_v4());
        channel.subscribe(subscriber, subscription_point);
        channel.publish(event);
        let (received_event_option, _receiver) = receiver.into_future().wait().ok().unwrap();
        assert!(received_event_option.is_some());
    }

    fn event(from_x: usize, from_y: usize, to_x: usize, to_y: usize) -> SpatialEvent{
        SpatialEvent{
            from: Point(from_x, from_y),
            to: Some(Point(to_x, to_y)),
            actor_id: Uuid::new_v4(),
            is_a_move: true,
        }
    }
}