extern crate bincode;
extern crate core;
extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate serde;
#[macro_use]extern crate serde_derive;
extern crate uuid;
extern crate spatiub;

mod entity;

use std::error::Error;
use log::LevelFilter;

fn main() -> Result<(), Box<Error>> {
    // Always print backtrace on panic.
    ::std::env::set_var("RUST_BACKTRACE", "1");

    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .filter_level(LevelFilter::Info)
        .init();

    info!("Hello, world!");
    Ok(())
}