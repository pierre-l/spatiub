extern crate clap;
extern crate core;
extern crate env_logger;
extern crate futures;
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
use log::LevelFilter;
use spatiub::spatial::MapDefinition;
use std::net::SocketAddr;
use clap::Arg;

mod client;

fn main() {
    setup_logging();

    let matches = App::new("Spatiub")
        .version("0.1")
        .author("Pierre L. <pierre.larger@gmail.com>")
        .arg(Arg::with_name("rate")
            .short("r")
            .long("message_rate")
            .value_name("RATE")
            .help("The approximate message rate per client")
            .takes_value(true))
        .arg(Arg::with_name("number_of_clients")
            .short("n")
            .long("number_of_clients")
            .value_name("NUMBER_OF_CLIENTS")
            .help("The number of clients to per core")
            .takes_value(true))
        .get_matches();

    let addr: SocketAddr = "127.0.0.1:6142".parse().unwrap();
    let map = MapDefinition::new(16, 1024 * 4);

    let msg_per_sec = matches.value_of("rate").unwrap_or("1").parse::<u64>().unwrap();
    info!("Message rate: {}", msg_per_sec);

    let number_of_clients = matches.value_of("number_of_clients").unwrap_or("1000").parse::<usize>().unwrap();
    info!("Number of clients: {}", number_of_clients);

    client::run_clients(
        &map,
        addr,
        number_of_clients,
        format!("client_log.csv").as_str(),
        msg_per_sec,
    );
}

fn setup_logging() {
// Always print backtrace on panic.
    ::std::env::set_var("RUST_BACKTRACE", "1");

    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .filter_level(LevelFilter::Info)
        .init();
}