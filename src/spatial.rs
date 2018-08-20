use pub_sub::PubSubChannel;
use pub_sub::Subscriber;
use std::collections::HashSet;
use std::rc::Rc;

type Channel<S> = PubSubChannel<S, SpatialEvent>;

struct SpatialChannel<S> where S: Subscriber<SpatialEvent>{
    map_definition: MapDefinition,
    channels: Vec<Channel<S>>,
}

impl <S> SpatialChannel<S> where S: Subscriber<SpatialEvent>{
    pub fn new(map_definition: MapDefinition)
        -> SpatialChannel<S>
    {
        let mut channels = vec![];

        for _x in 0..map_definition.map_width_in_zones {
            for _y in 0..map_definition.map_width_in_zones {
                channels.push(Channel::new());
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

        compute_indexes_for_zones_in_range(&event.to, zone_width, |index|{
            if !from_indexes.contains(&index) {
                self.publish_if_channel_exists(index, &event);
            }
        });
    }

    pub fn subscribe(&mut self, subscriber: S, position: &Point) {
        let zone_index = zone_index_for_point(position, self.map_definition.zone_width);
        if let Some(channel) = self.channels.get_mut(zone_index) {
            channel.subscribe(subscriber);
        } else {
            panic!()
        }
    }

    fn publish_if_channel_exists(&mut self, channel_index: usize, event: &Rc<SpatialEvent>) {
        if let Some(channel) =  self.channels.get_mut(channel_index) {
            if let Err(err) = channel.publish(event.clone()) {
                error!("{}", err)
            }
        }
    }
}

struct SpatialEvent{
    from: Point,
    to: Point,
}

struct MapDefinition{
    zone_width: usize,
    map_width_in_zones: usize
}

#[derive(Debug, Hash, Eq, PartialEq)]
struct Point(usize, usize);

#[derive(Debug, Hash, Eq, PartialEq)]
struct Zone(Point, Point);

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
        assert_can_subscribe(Point(0, 0), Point(1, 0));
        assert_can_subscribe(Point(ZONE_WIDTH, 0), Point(ZONE_WIDTH+1, 0));
        assert_can_subscribe(Point(ZONE_WIDTH, ZONE_WIDTH), Point(ZONE_WIDTH, ZONE_WIDTH+1));
    }

    fn assert_can_subscribe(from: Point, to: Point) {
        let mut channel = SpatialChannel::new(
            MapDefinition {
                zone_width: ZONE_WIDTH,
                map_width_in_zones: ZONE_WIDTH,
            }
        );

        let event = SpatialEvent {
            from,
            to,
        };

        let (subscriber, receiver) = futures_sub::new_subscriber();
        channel.subscribe(subscriber, &event.from);
        channel.publish(event);
        let (received_event_option, _receiver) = receiver.into_future().wait().ok().unwrap();
        assert!(received_event_option.is_some());
    }
}