extern crate core;
extern crate futures;

use std::error::Error;

mod pub_sub;
mod std_sub;
mod futures_sub;

fn main() -> Result<(), Box<Error>> {
    println!("Hello, world!");
    Ok(())
}