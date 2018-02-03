#![feature(use_extern_macros, entry_or_default)]
extern crate strum;
#[macro_use]
extern crate strum_macros;

mod client;
mod model;
mod consts;

fn main() {
    let mut client = client::Client::new();
    client.run();
}
