extern crate core;

use std::error::Error;

mod pub_sub;
mod std_sub;

fn main() -> Result<(), Box<Error>> {
    println!("Hello, world!");
    Ok(())
}