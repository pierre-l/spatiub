extern crate bincode;
extern crate bytes;
extern crate core;
extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate serde;
#[macro_use]extern crate serde_derive;
extern crate spatiub;
extern crate tokio;
extern crate tokio_codec;
extern crate uuid;

use log::LevelFilter;
use std::thread;
use std::net::SocketAddr;
use std::time::Duration;

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


    let addr: SocketAddr = "127.0.0.1:6142".parse().unwrap();
    let addr_clone = addr.clone();
    thread::spawn(move ||{
        server::server(&addr_clone);
        info!("Server stopped");
    });

    thread::sleep(Duration::from_millis(100));
    client::run_client(&addr)
}