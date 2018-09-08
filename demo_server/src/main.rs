extern crate bincode;
extern crate bytes;
extern crate core;
extern crate env_logger;
extern crate futures;
extern crate hwloc;
extern crate libc;
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
use futures::{future, Future, stream, Stream};
use hwloc::{CPUBIND_THREAD, CpuSet, ObjectType, Topology};
use log::LevelFilter;
use message::Message;
use rand::Rng;
use rand::thread_rng;
use rand::ThreadRng;
use spatiub::spatial::Entity;
use spatiub::spatial::MapDefinition;
use spatiub::spatial::Point;
use spatiub::spatial::SpatialEvent;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::ops::Add;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;
use tokio::runtime::current_thread::Runtime;
use tokio::timer::Delay;
use uuid::Uuid;

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

    let hw_topo = Arc::new(Mutex::new(Topology::new()));

    let num_cores = {
        let topo_locked = hw_topo.lock().unwrap();
        (*topo_locked).objects_with_type(&ObjectType::Core).unwrap().len()
    };
    info!("Found {} cores.", num_cores);

    let map = MapDefinition::new(16, 1024 * 4);

    let addr: SocketAddr = "127.0.0.1:6142".parse().unwrap();
    spawn_server_thread(hw_topo.clone(), map.clone(), addr.clone());

    let mut client_handles = vec![];
    for i in 1..num_cores {
        let handle = spawn_client_thread(hw_topo.clone(), i, map.clone(), addr.clone(), 1000);
        client_handles.push(handle);
    }

    for handle in client_handles{
        handle.join().unwrap();
    }
}

fn spawn_client_thread(
    hw_topo: Arc<Mutex<Topology>>,
    cpu_index: usize,
    map: MapDefinition,
    addr: SocketAddr,
    number_of_clients: usize
) -> JoinHandle<()>{
    let handle = thread::spawn(move || {
        pin_thread_to_core(hw_topo, cpu_index);

        run_clients(map, addr, number_of_clients);
        info!("Clients stopped");
    });

    handle
}

fn run_clients(
    map: MapDefinition,
    addr: SocketAddr,
    number_of_clients: usize
) {

    let mut iter = vec![];
    for _i in 0..number_of_clients { iter.push(()) }

    let clients = stream::iter_ok(iter)
        .map(|_i| {
            let ref addr = addr;
            let client_entity_id = RefCell::new(None);
            let map = map.clone();
            client::client(
                &addr,
                move |message| {
                    if let Message::ConnectionAck(entity) = &message {
                        client_entity_id.replace(Some(entity.id().clone()));
                    } else if let Message::Event(event) = &message {
                        if let Some(ref destination) = &event.to {
                            let latency = event.acting_entity.last_state_update.elapsed();

                            let latency = latency.subsec_nanos();

                            if latency > 10_000_000 {
                                info!("Position: {:?}, Latency: {}", destination, latency);
                            }
                        }
                    };

                    if let Some(ref entity_id) = &*client_entity_id.borrow() {
                        trigger_new_move_if_client_entity_involved(message, &map, entity_id)
                    } else {
                        panic!("Expected the entity id to be set");
                    }
                })
        })
        .buffered(number_of_clients);

    let mut runtime = Runtime::new().unwrap();
    if let Err(_err) = runtime.block_on(
        clients.for_each(|_| {
            future::ok(())
        })
    ) {
        info!("Client stopped");
    }

    drop(addr);
}

fn spawn_server_thread(
    hw_topo: Arc<Mutex<Topology>>,
    map: MapDefinition,
    addr_clone: SocketAddr,
) -> JoinHandle<()>{
    let handle = thread::spawn(move || {
        pin_thread_to_core(hw_topo, 0);

        server::server(&addr_clone, map);
        info!("Server stopped");
    });

    thread::sleep(Duration::from_millis(5_000));

    handle
}

fn trigger_new_move_if_client_entity_involved(
    message: Message,
    map: &MapDefinition,
    client_entity_id: &Uuid,
)
    -> Option<impl Future<Item=Message, Error=()>>
{
    // PERFORMANCE Suboptimal. Is there a way to avoid calling thread_rng everytime?
    let mut rng = thread_rng();

    if let Message::Event(
        SpatialEvent{
            from: _,
            to: Some(to),
            acting_entity,
            is_a_move: true,
        }
    ) = message{
        if acting_entity.id() == client_entity_id {
            let delayed_move = trigger_new_move(&mut rng, &map, acting_entity, to);

            Some(delayed_move)
        } else {
            None
        }
    } else {
        None
    }
}

fn trigger_new_move(rng: &mut ThreadRng, map: &MapDefinition, mut entity: DemoEntity, from: Point) -> impl Future<Item=Message, Error=()> {
    let next_destination = map.random_point_next_to(&from, rng);
    Delay::new(Instant::now().add(Duration::from_millis(rng.gen_range(500, 1500))))
        .map(move |()| {
            entity.last_state_update = Timestamp::new();

            Message::Event(SpatialEvent {
                from,
                to: Some(next_destination),
                acting_entity: entity,
                is_a_move: true,
            })
        })
        .map_err(|err|{
            panic!("Timer error: {}", err)
        })
}

fn cpuset_for_core(topology: &Topology, idx: usize) -> CpuSet {
    let cores = (*topology).objects_with_type(&ObjectType::Core).unwrap();
    match cores.get(idx) {
        Some(val) => val.cpuset().unwrap(),
        None => panic!("No Core found with id {}", idx)
    }
}

fn pin_thread_to_core(hw_topo: Arc<Mutex<Topology>>, cpu_index: usize) {
    let tid = unsafe { libc::pthread_self() };
    let mut locked_topo = hw_topo.lock().unwrap();
    let bind_to = cpuset_for_core(&*locked_topo, cpu_index);
    locked_topo.set_cpubind_for_thread(tid, bind_to, CPUBIND_THREAD).unwrap();
}