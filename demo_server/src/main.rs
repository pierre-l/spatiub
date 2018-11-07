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

mod server;

fn main() {
    setup_logging();

    let matches = App::new("Spatiub")
        .version("0.1")
        .author("Pierre L. <pierre.larger@gmail.com>")
        .arg(Arg::with_name("bind_address")
            .short("b")
            .long("bind_address")
            .value_name("BIND_ADDRESS")
            .help("The address to bind the server to")
            .takes_value(true))
        .get_matches();

    let addr = matches.value_of("bind_address")
        .unwrap_or("127.0.0.1:6142").parse::<SocketAddr>().unwrap();
    let map = MapDefinition::new(16, 1024 * 4);

    server::server(&addr, &map);
}

fn setup_logging() {
// Always print backtrace on panic.
    ::std::env::set_var("RUST_BACKTRACE", "1");

    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .filter_level(LevelFilter::Info)
        .init();
}