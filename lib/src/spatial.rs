use pub_sub::Subscriber;
use rand::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use uuid::Uuid;

pub struct SpatialChannel<S, E> where S: Subscriber<SpatialEvent<E>>, E: Entity+Clone {
    map_definition: MapDefinition,
    channels: Vec<ZoneChannel<S, E>>,
}

impl <S, E> SpatialChannel<S, E> where S: Subscriber<SpatialEvent<E>>, E: Entity+Clone{
    pub fn new(map_definition: MapDefinition)
               -> SpatialChannel<S, E>
    {
        let mut channels = vec![];

        let zone_width = map_definition.zone_width;
        let map_width_in_zones = map_definition.map_width_in_zones;

        for x in 0..map_width_in_zones {
            for y in 0..map_width_in_zones {
                let area_start = Point(x * zone_width, y * zone_width);
                let area_end = Point(area_start.0 + zone_width, area_start.1 + zone_width);
                let area = Zone(area_start, area_end);

                channels.push(ZoneChannel::new(area));
            }
        }

        SpatialChannel{
            channels,
            map_definition,
        }
    }

    pub fn publish(&mut self, event: SpatialEvent<E>) {
        debug!("Publishing {}: {:?} => {:?}", event.acting_entity.id(), event.from, event.to);
        let event = Rc::new(event);
        let zone_width = self.map_definition.zone_width;
        let map_width_in_zones = self.map_definition.map_width_in_zones;

        // Publish in the areas that were already in range.
        let mut from_indexes = HashSet::new();
        let mut entity_subscription_cell: RefCell<Option<S>> = RefCell::new(None);
        compute_indexes_for_zones_in_range(&event.from, zone_width, map_width_in_zones, |index|{
            from_indexes.insert(index);

            if let Some(channel) =  self.channels.get_mut(index) {
                if let Some(dropped_subscription) = channel.publish(event.clone()) {
                    entity_subscription_cell.replace(Some(dropped_subscription));
                }
            };
        });

        if let Some(ref destination) = event.to {
            // Publish in the areas that are now in range.
            compute_indexes_for_zones_in_range(destination, zone_width, map_width_in_zones, |index|{
                if !from_indexes.contains(&index) { // Exclude the zones that were already in range.
                    if let Some(channel) =  self.channels.get_mut(index) {
                        if let Some(_dropped_subscription) = channel.publish(event.clone()){
                            panic!() // No subscription should be dropped in the new areas in visible range.
                        }

                        if let Some(dropped_subscriber) = entity_subscription_cell.get_mut() {
                            channel.for_each_entity_in_zone(|entity, position|{
                                let entity_in_zone_event = SpatialEvent{
                                    from: position.clone(),
                                    to: Some(position.clone()),
                                    acting_entity: entity.clone(),
                                    is_a_move: false,
                                };

                                let _res = // Nothing to do if it fails, result is ignored.
                                    dropped_subscriber.send(Rc::new(entity_in_zone_event));
                            })
                        }
                    }
                }
            });

            if let Some(dropped_subscriber) = entity_subscription_cell.replace(None) {
                self.do_subscribe(dropped_subscriber, destination, false);
            } else {
                // TODO Panic? Requires a change in the API because it means every entity.rs has a matching subscription.
            }
        }
    }

    pub fn subscribe(&mut self, subscriber: S, position: &Point) {
        self.do_subscribe(subscriber, position, true);
    }

    fn do_subscribe(&mut self, subscriber: S, position: &Point, warn_of_entities_in_zone: bool) {
        let zone_index = zone_index_for_point(position, &self.map_definition);
        if let Some(channel) = self.channels.get_mut(zone_index) {
            channel.subscribe(subscriber, warn_of_entities_in_zone);
        } else {
            panic!()
        }
    }
}

pub struct ZoneChannel<S, E> where S: Subscriber<SpatialEvent<E>>, E: Entity+Clone{
    area: Zone,
    subscribers: Vec<S>,
    entities_in_zone: HashMap<Uuid, (Point, E)>,
}

impl <S, E> ZoneChannel<S, E> where S: Subscriber<SpatialEvent<E>>, E: Entity+Clone {
    pub fn new(area: Zone) -> ZoneChannel<S, E> {
        ZoneChannel{
            area,
            subscribers: vec![],
            entities_in_zone: HashMap::new(),
        }
    }

    pub fn subscribe(&mut self, subscriber: S, warn_of_entities_in_zone: bool) {
        if warn_of_entities_in_zone{
            for (position, entity) in self.entities_in_zone.values(){
                match subscriber.send(Rc::new(SpatialEvent{
                    from: position.clone(),
                    to: Some(position.clone()),
                    acting_entity: entity.clone(),
                    is_a_move: false,
                })) {
                    Ok(keep) => {
                        if !keep {
                            panic!("This is not an expected behavior to subscribe with an subscriber that drops immediately.")
                        }
                    },
                    Err(err) => {
                        panic!("The subscriber should still be valid when subscribing. Cause: {}", err)
                    }
                }
            }
        }

        debug!("Entity {} subscribing to zone {:?}", subscriber.entity_id(), self.area);
        self.subscribers.push(subscriber);
    }

    pub fn publish(&mut self, event: Rc<SpatialEvent<E>>) -> Option<S>{
        let leaves_the_zone = if event.is_a_move {
            if self.area.point_is_in(&event.from){
                if let Some(ref destination) = &event.to {
                    if self.area.point_is_not_in(destination) {
                        self.entities_in_zone.remove(event.acting_entity.id());
                        true
                    } else {
                        self.insert_entity(event.acting_entity.clone(), destination.clone());
                        false
                    }
                } else {
                    self.entities_in_zone.remove(event.acting_entity.id());
                    true
                }
            } else {
                if let Some(ref destination) = &event.to {
                    if self.area.point_is_in(destination) {
                        self.insert_entity(event.acting_entity.clone(), destination.clone());
                    }
                    false
                } else {
                    false
                }
            }
        } else {
            false
        };

        if leaves_the_zone {
            debug!("Entity {} leaving zone {:?}", event.acting_entity.id(), self.area);
        }

        let mut dropped_subscriber_option = None;
        self.subscribers.retain(|subscriber|{
            match subscriber.send(event.clone()) {
                Ok(retain) => {
                    if leaves_the_zone && subscriber.entity_id() == event.acting_entity.id() {
                        dropped_subscriber_option = Some(subscriber.clone());

                        false
                    } else {
                        retain
                    }
                },
                Err(_err) => {
                    false
                }
            }
        });

        dropped_subscriber_option
    }

    fn insert_entity(&mut self, entity: E, position: Point) {
        let entity_id = entity.id().clone();
        self.entities_in_zone.insert(entity_id, (position, entity));
    }

    fn for_each_entity_in_zone<C>(&mut self, consumer: C) where C: Fn(&mut E, &Point) {
        self.entities_in_zone.retain(|_id, (position, entity)|{
            consumer(entity, position);
            true
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpatialEvent<E: Entity>{
    pub from: Point,
    pub to: Option<Point>,
    pub acting_entity: E,
    pub is_a_move: bool,
}

#[derive(Debug, Clone)]
pub struct MapDefinition{
    zone_width: usize,
    map_width_in_zones: usize,
    coordinate_max_value: usize,
}

impl MapDefinition{
    pub fn new(zone_width: usize, map_width_in_zones: usize) -> MapDefinition{
        MapDefinition{
            coordinate_max_value: map_width_in_zones * zone_width - 1,
            zone_width,
            map_width_in_zones,
        }
    }

    pub fn point_is_inside(&self, point: &Point) -> bool {
        self.coord_is_inside(&point.0) && self.coord_is_inside(&point.1)
    }

    pub fn coord_is_inside(&self, coord: &usize) -> bool {
        coord < &(&self.zone_width * &self.map_width_in_zones)
    }

    pub fn random_point(&self, rng: &mut ThreadRng) -> Point {
        Point(rng.gen_range(0, self.coordinate_max_value), rng.gen_range(0, self.coordinate_max_value))
    }

    pub fn random_point_next_to(&self, point: &Point, rng: &mut ThreadRng) -> Point {
        let mut candidate = point.clone();

        let direction = rng.gen_range(0, 4);

        // Doing it this way avoids the need of a loop.
        match direction {
            0 => {
                if candidate.0 < self.coordinate_max_value {
                    candidate.0 += 1;
                } else if candidate.0 > 0 {
                    candidate.0 -= 1;
                } else if candidate.1 < self.coordinate_max_value {
                    candidate.1 += 1;
                } else {
                    candidate.1 -= 1;
                }
            },
            1 => {
                if candidate.0 > 0 {
                    candidate.0 -= 1;
                } else if candidate.1 < self.coordinate_max_value {
                    candidate.1 += 1;
                } else if candidate.1 > 0 {
                    candidate.1 -= 1;
                } else {
                    candidate.0 += 1;
                }
            },
            2 => {
                if candidate.1 < self.coordinate_max_value {
                    candidate.1 += 1;
                } else if candidate.1 > 0 {
                    candidate.1 -= 1;
                } else if candidate.0 < self.coordinate_max_value {
                    candidate.0 += 1;
                } else {
                    candidate.0 -= 1;
                }
            },
            3 => {
                if candidate.1 > 0 {
                    candidate.1 -= 1;
                } else if candidate.0 < self.coordinate_max_value {
                    candidate.0 += 1;
                } else if candidate.0 > 0 {
                    candidate.0 -= 1;
                } else {
                    candidate.1 += 1;
                }
            },
            _ => panic!() // Should not happen.
        };

        candidate
    }
}

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq, Clone)]
pub struct Point(pub usize, pub usize);

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Zone(Point, Point);

impl Zone{
    fn point_is_in(&self, point: &Point) -> bool {
        point.0 >= (self.0).0 && point.1 >= (self.0).1
            && point.0 < (self.1).0 && point.1 < (self.1).1
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
    map_width_in_zones: usize,
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
            let channel_index = (start_x + x_offset) * map_width_in_zones + (start_y + y_offset);
            consumer(channel_index);
        }
    }
}

fn zone_index_for_point(point: &Point, map_definition: &MapDefinition) -> usize{
    let x = point.0 / map_definition.zone_width ;
    let y = point.1 / map_definition.zone_width;
    x * map_definition.map_width_in_zones + y
}

#[cfg(test)]
mod tests{
    use env_logger;
    use pub_sub::PubSubError;
    use std::iter::FromIterator;
    use std::sync::Mutex;
    use super::*;
    use std::cmp::max;
    use std::cmp::min;

    const ZONE_WIDTH: usize = 16;
    const MAP_WIDTH_IN_ZONES: usize = 16;

    #[test]
    pub fn can_compute_random_points(){
        let map = MapDefinition::new(ZONE_WIDTH, MAP_WIDTH_IN_ZONES);

        let mut rng = thread_rng();
        for _i in 0..1_000_000{
            let point = map.random_point(&mut rng);

            assert!(map.point_is_inside(&point));
        }
    }

    #[test]
    pub fn can_compute_random_points_next_to(){
        let map = MapDefinition::new(ZONE_WIDTH, MAP_WIDTH_IN_ZONES);

        let mut rng = thread_rng();
        for _i in 0..1_000{
            let origin = map.random_point(&mut rng);
            let point = map.random_point_next_to(&origin, &mut rng);

            assert!(map.point_is_inside(&point), format!("Point {:?} outside of {:?}", point, map));

            let distance_x = max(origin.0, point.0) - min(origin.0, point.0);
            let distance_y = max(origin.1, point.1) - min(origin.1, point.1);
            let distance = distance_x * distance_x + distance_y * distance_y;
            assert_eq!(1, distance);
        }
    }

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
        env_logger::init();
        let mut channel = test_channel();

        let entity_id = Uuid::new_v4();
        let entity = TestEntity{
            id: entity_id.clone(),
        };

        let subscriber = CountingSubscriber::new(entity_id);

        let mut position = Point(0, 0);
        channel.subscribe(subscriber.clone(), &position);

        let number_of_events = ZONE_WIDTH * 10;
        for _i in 0..number_of_events {
            let destination = Point(position.0 + 1, position.1);

            channel.publish(SpatialEvent{
                from: position,
                to: Some(destination.clone()),
                acting_entity: entity.clone(),
                is_a_move: true,
            });

            position = destination;
        }

        assert_eq!(number_of_events, subscriber.number_of_events_received());
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
    pub fn can_compute_indexes_for_zones_in_range(){
        let expected = HashSet::from_iter(vec![
            0, 1, ZONE_WIDTH, ZONE_WIDTH +1,
        ]);

        let mut found = HashSet::new();
        compute_indexes_for_zones_in_range(&Point(0, 0), ZONE_WIDTH, MAP_WIDTH_IN_ZONES, |index|{
            found.insert(index);
        });

        assert_eq!(expected, found);

        let expected = HashSet::from_iter(vec![
            0, 1, ZONE_WIDTH, ZONE_WIDTH +1,
        ]);

        let mut found = HashSet::new();
        compute_indexes_for_zones_in_range(&Point(16, 0), ZONE_WIDTH, MAP_WIDTH_IN_ZONES, |index|{
            found.insert(index);
        });

        assert_eq!(expected, found);
    }

    #[test]
    pub fn new_subscriber_is_warned_of_existing_entities() {
        let mut channel = test_channel();

        channel.publish(event(0, 0, 1, 0));

        let subscriber = CountingSubscriber::new(Uuid::new_v4());
        channel.subscribe(subscriber.clone(), &Point(0, 0));

        assert_eq!(1, subscriber.number_of_events_received());
    }

    #[test]
    pub fn moving_entity_is_warned_of_entities_now_in_range() {
        let mut channel = test_channel();

        channel.publish(event(0, 0, 1, 0));

        let entity_id = Uuid::new_v4();
        let entity_position = Point(ZONE_WIDTH - 1, ZONE_WIDTH - 1);
        let subscriber = CountingSubscriber::new(entity_id.clone());
        channel.subscribe(subscriber.clone(), &entity_position);

        channel.publish(SpatialEvent{
            to: Some(Point(entity_position.0 + 1, entity_position.1)),
            from: entity_position,
            acting_entity: TestEntity{
                id: entity_id
            },
            is_a_move: true,
        });

        assert_eq!(2, subscriber.number_of_events_received());
    }

    fn assert_can_subscribe(subscription_point: &Point, event: SpatialEvent<TestEntity>) {
        let mut channel = test_channel();
        let subscriber = CountingSubscriber::new(Uuid::new_v4());
        channel.subscribe(subscriber.clone(), subscription_point);
        channel.publish(event);

        assert_eq!(1, subscriber.number_of_events_received())
    }

    fn test_channel() -> SpatialChannel<CountingSubscriber, TestEntity> {
        SpatialChannel::new(
            MapDefinition::new(ZONE_WIDTH, ZONE_WIDTH)
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

    #[derive(Clone)]
    struct CountingSubscriber{
        entity_id: Uuid,
        number_of_events_received: Rc<Mutex<usize>>,
    }

    impl CountingSubscriber{
        pub fn new(entity_id: Uuid) -> CountingSubscriber{
            CountingSubscriber{
                entity_id,
                number_of_events_received: Rc::new(Mutex::new(0)),
            }
        }

        fn number_of_events_received(&self) -> usize {
            match self.number_of_events_received.lock(){
                Ok(number) => {
                    *number
                },
                Err(_err) => panic!()
            }
        }
    }

    impl Subscriber<SpatialEvent<TestEntity>> for CountingSubscriber{
        fn send(&self, _event: Rc<SpatialEvent<TestEntity>>) -> Result<bool, PubSubError> {
            match self.number_of_events_received.lock(){
                Ok(mut number) => {
                    *number += 1
                },
                Err(_err) => {
                    panic!()
                }
            }
            Ok(true)
        }

        fn entity_id(&self) -> &Uuid {
            &self.entity_id
        }
    }
}