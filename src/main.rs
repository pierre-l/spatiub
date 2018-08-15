extern crate core;
extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;

use std::error::Error;

mod pub_sub;
mod std_sub;
mod futures_sub;

fn main() -> Result<(), Box<Error>> {
    println!("Hello, world!");
    Ok(())
}