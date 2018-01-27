#![feature(use_extern_macros)]

mod client;
mod model;
mod consts;

fn main() {
    let mut client = client::Client::new();
    client.run();
}
