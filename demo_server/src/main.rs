extern crate core;
extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate uuid;

use std::error::Error;

fn main() -> Result<(), Box<Error>> {
    println!("Hello, world!");
    Ok(())
}