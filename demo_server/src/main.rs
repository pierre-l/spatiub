extern crate clap;
extern crate core;
extern crate env_logger;
extern crate futures;
extern crate hwloc;
extern crate libc;
#[macro_use] extern crate log;
extern crate rand;
extern crate serde;
extern crate spatiub;
extern crate tokio;
extern crate tokio_codec;
extern crate uuid;
extern crate spatiub_demo_core;

use clap::App;
use hwloc::{CPUBIND_THREAD, CpuSet, ObjectType, Topology};
use log::LevelFilter;
use spatiub::spatial::MapDefinition;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use clap::Arg;

mod server;

fn main() {
    setup_logging();

    let matches = App::new("Spatiub")
        .arg(Arg::with_name("core")
            .short("c")
            .long("core")
            .value_name("CORE")
            .help("The logical core (or processing unit) to pin the server on.")
            .takes_value(true))
        .version("0.1")
        .author("Pierre L. <pierre.larger@gmail.com>")
        .get_matches();

    let hw_topo = Arc::new(Mutex::new(Topology::new()));
    let addr: SocketAddr = "127.0.0.1:6142".parse().unwrap();
    let map = MapDefinition::new(16, 1024 * 4);

    let core = matches.value_of("core").unwrap_or("0").parse::<usize>().unwrap();
    info!("Core: {}", core);

    let addr = addr.clone();
    let map = map.clone();
    run_thread(
        hw_topo.clone(),
        core,
        "server".to_string(),move || server::server(&addr, &map),
    ).join().unwrap();
}

fn setup_logging() {
// Always print backtrace on panic.
    ::std::env::set_var("RUST_BACKTRACE", "1");

    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .filter_level(LevelFilter::Info)
        .init();
}

fn run_thread(
    hw_topo: Arc<Mutex<Topology>>,
    cpu_index: usize,
    label: String,
    task: impl Fn() + Send + 'static,
) -> JoinHandle<()>{
    info!("Spawning {} on core #{}", label, cpu_index);

    thread::spawn(move || {
        pin_thread_to_core(hw_topo, cpu_index);

        thread::sleep(Duration::from_micros(900));

        task();
        info!("{} stopped", label);
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