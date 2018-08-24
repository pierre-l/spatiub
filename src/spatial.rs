use pub_sub::Subscriber;
use std::collections::HashSet;
use std::rc::Rc;
use uuid::Uuid;
use futures_sub::FutureSubscriber;
use std::collections::HashMap;

pub struct SpatialChannel<E> where E: Entity+Clone {
    map_definition: MapDefinition,
    channels: Vec<ZoneChannel<E>>,
}

impl <E> SpatialChannel<E> where E: Entity+Clone{
    pub fn new(map_definition: MapDefinition)
        -> SpatialChannel<E>
    {
        let mut channels = vec![];

        let zone_width = map_definition.zone_width;
        let map_width_in_zones = map_definition.map_width_in_zones;

        for x in 0..map_width_in_zones {
            for y in 0..map_width_in_zones {
                let visible_area = compute_visible_area(zone_width, map_width_in_zones, x, y);

                channels.push(ZoneChannel::new(visible_area));
            }
        }

        SpatialChannel{
            channels,
            map_definition,
        }
    }

    pub fn publish(&mut self, event: SpatialEvent<E>) {
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

    pub fn subscribe(&mut self, subscriber: FutureSubscriber<SpatialEvent<E>>, position: &Point) {
        let zone_index = zone_index_for_point(position, self.map_definition.zone_width);
        if let Some(channel) = self.channels.get_mut(zone_index) {
            channel.subscribe(subscriber);
        } else {
            panic!()
        }
    }

    fn publish_if_channel_exists(&mut self, channel_index: usize, event: &Rc<SpatialEvent<E>>) {
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

pub struct ZoneChannel<E> where E: Entity+Clone{
    visible_area: Zone,
    subscribers: Vec<FutureSubscriber<SpatialEvent<E>>>,
    visible_entities: HashMap<Uuid, (Point, E)>,
}

impl <E> ZoneChannel<E> where E: Entity+Clone {
    pub fn new(zone: Zone) -> ZoneChannel<E> {
        ZoneChannel{
            visible_area: zone,
            subscribers: vec![],
            visible_entities: HashMap::new(),
        }
    }

    pub fn subscribe(&mut self, subscriber: FutureSubscriber<SpatialEvent<E>>) {
        for (position, entity) in self.visible_entities.values(){
            subscriber.send(Rc::new(SpatialEvent{
                from: position.clone(),
                to: Some(position.clone()),
                acting_entity: entity.clone(),
                is_a_move: false,
            }));
        }

        self.subscribers.push(subscriber);
    }

    pub fn publish(&mut self, event: Rc<SpatialEvent<E>>) -> Option<FutureSubscriber<SpatialEvent<E>>>{
        self.process_entity_move(&event);

        let mut dropped_subscriber_option = None;

        self.subscribers.retain(|subscriber|{
            match subscriber.send(event.clone()) {
                Ok(retain) => {
                    if event.is_a_move && subscriber.entity_id() == event.acting_entity.id() {
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

    fn process_entity_move(&mut self, event: &Rc<SpatialEvent<E>>) {
        if event.is_a_move {
            if self.visible_area.point_is_in(&event.from) {
                if let Some(ref destination) = event.to {
                    if self.visible_area.point_is_in(&destination) {
                        let entity = event.acting_entity.clone();
                        let entity_id = entity.id().clone();
                        let position = destination.clone();
                        self.visible_entities.insert(entity_id, (position, entity));
                    } else {
                        self.visible_entities.remove(event.acting_entity.id());
                    }
                } else {
                    self.visible_entities.remove(event.acting_entity.id());
                }
            } else if let Some(ref destination) = event.to {
                if self.visible_area.point_is_in(&destination) {
                    let entity = event.acting_entity.clone();
                    let entity_id = entity.id().clone();
                    let position = destination.clone();
                    self.visible_entities.insert(entity_id, (position, entity));
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct SpatialEvent<E: Entity>{
    from: Point,
    to: Option<Point>,
    acting_entity: E,
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

impl Zone{
    fn point_is_in(&self, point: &Point) -> bool {
        (self.0).0 <= point.0 && (self.0).1 <= point.1
            && (self.1).0 > point.0 && (self.1).1 > point.1
    }

    fn point_is_not_in(&self, point: &Point) -> bool {
        !self.point_is_in(point)
    }
}

pub trait Entity {
    fn id(&self) -> &Uuid;
}

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

const RANGE_IN_ZONES: usize = 1;
fn compute_visible_area(zone_width: usize, map_width_in_zones: usize, x: usize, y: usize) -> Zone {
    let mut visible_area_start = Point(x * zone_width, y * zone_width);
    let mut visible_area_end = Point(visible_area_start.0 + zone_width, visible_area_start.1 + zone_width);

    visible_area_start.0 = if visible_area_start.0 >= zone_width {
        visible_area_start.0 - zone_width * RANGE_IN_ZONES
    } else {
        visible_area_start.0
    };

    visible_area_start.1 = if visible_area_start.1 >= zone_width {
        visible_area_start.1 - zone_width * RANGE_IN_ZONES
    } else {
        visible_area_start.1
    };

    visible_area_end.0 = if visible_area_end.0 / zone_width < map_width_in_zones {
        visible_area_end.0 + zone_width * RANGE_IN_ZONES
    } else {
        visible_area_end.0
    };

    visible_area_end.1 = if visible_area_end.1 / zone_width < map_width_in_zones {
        visible_area_end.1 + zone_width * RANGE_IN_ZONES
    } else {
        visible_area_end.1
    };

    Zone(visible_area_start, visible_area_end)
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
        let mut channel = test_channel();

        let entity_id = Uuid::new_v4();
        let entity = TestEntity{
            id: entity_id.clone(),
        };

        let (subscriber, mut receiver) = futures_sub::new_subscriber(entity_id);

        let mut position = Point(0, 0);
        channel.subscribe(subscriber, &position);

        for _i in 0..ZONE_WIDTH * 10 {
            let destination = Point(position.0 + 1, position.1);

            channel.publish(SpatialEvent{
                from: position,
                to: Some(destination.clone()),
                acting_entity: entity.clone(),
                is_a_move: true,
            });

            let (received_event_option, receiver_tmp) = receiver.into_future().wait().ok().unwrap();
            receiver = receiver_tmp;

            assert!(received_event_option.is_some());
            position = destination;
        }
    }

    #[test]
    pub fn can_compute_if_point_is_in_zone() {
        let zone = Zone(Point(ZONE_WIDTH, ZONE_WIDTH), Point(ZONE_WIDTH*2, ZONE_WIDTH*2));

        assert!(zone.point_is_in(&Point(ZONE_WIDTH, ZONE_WIDTH)));
        assert!(zone.point_is_in(&Point(ZONE_WIDTH+1, ZONE_WIDTH)));
        assert!(zone.point_is_in(&Point(ZONE_WIDTH, ZONE_WIDTH+1)));
        assert!(zone.point_is_not_in(&Point(ZONE_WIDTH*2, ZONE_WIDTH)));
        assert!(zone.point_is_not_in(&Point(ZONE_WIDTH, ZONE_WIDTH*2)));
        assert!(zone.point_is_not_in(&Point(ZONE_WIDTH*2, ZONE_WIDTH*2)));
        assert!(zone.point_is_not_in(&Point(ZONE_WIDTH-1, ZONE_WIDTH)));
        assert!(zone.point_is_not_in(&Point(ZONE_WIDTH, ZONE_WIDTH-1)));
        assert!(zone.point_is_not_in(&Point(0, 0)));
    }

    #[test]
    pub fn can_compute_visible_area() {
        let zone = compute_visible_area(ZONE_WIDTH, 3, 1, 1);
        assert_eq!(Zone(Point(0, 0), Point(ZONE_WIDTH*3, ZONE_WIDTH*3)), zone);

        let zone = compute_visible_area(ZONE_WIDTH, 2, 1, 1);
        assert_eq!(Zone(Point(0, 0), Point(ZONE_WIDTH*2, ZONE_WIDTH*2)), zone);

        let zone = compute_visible_area(ZONE_WIDTH, 3, 0, 0);
        assert_eq!(Zone(Point(0, 0), Point(ZONE_WIDTH*2, ZONE_WIDTH*2)), zone);
    }

    #[test]
    pub fn new_subscriber_is_warned_of_existing_entities() {
        let mut channel = test_channel();

        channel.publish(event(0, 0, 1, 0));

        let (subscriber, receiver) = futures_sub::new_subscriber(Uuid::new_v4());
        channel.subscribe(subscriber, &Point(0, 0));
        let (received_event_option, _receiver) = receiver.into_future().wait().ok().unwrap();
        assert!(received_event_option.is_some());
    }

    #[test]
    pub fn moving_entity_is_warned_of_entities_now_in_range() {
        let mut channel = test_channel();

        channel.publish(event(0, 0, 1, 0));

        let entity_id = Uuid::new_v4();
        let entity_position = Point(ZONE_WIDTH - 1, ZONE_WIDTH - 1);
        let (subscriber, receiver) = futures_sub::new_subscriber(entity_id.clone());
        channel.subscribe(subscriber, &entity_position);

        channel.publish(SpatialEvent{
            to: Some(Point(entity_position.0 + 1, entity_position.1)),
            from: entity_position,
            acting_entity: TestEntity{
                id: entity_id
            },
            is_a_move: true,
        });

        let (_, receiver_rest) = receiver.into_future().wait().ok().unwrap();
        let (received_event_option, _receiver_rest) = receiver_rest.into_future().wait().ok().unwrap();
        assert!(received_event_option.is_some());
    }

    fn assert_can_subscribe(subscription_point: &Point, event: SpatialEvent<TestEntity>) {
        let mut channel = test_channel();
        let (subscriber, receiver) = futures_sub::new_subscriber(Uuid::new_v4());
        channel.subscribe(subscriber, subscription_point);
        channel.publish(event);
        let (received_event_option, _receiver) = receiver.into_future().wait().ok().unwrap();
        assert!(received_event_option.is_some());
    }

    fn test_channel() -> SpatialChannel<TestEntity> {
        SpatialChannel::new(
            MapDefinition {
                zone_width: ZONE_WIDTH,
                map_width_in_zones: ZONE_WIDTH,
            }
        )
    }

    fn event(from_x: usize, from_y: usize, to_x: usize, to_y: usize) -> SpatialEvent<TestEntity>{
        SpatialEvent{
            from: Point(from_x, from_y),
            to: Some(Point(to_x, to_y)),
            acting_entity: TestEntity{
                id: Uuid::new_v4()
            },
            is_a_move: true,
        }
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
}