#![feature(use_extern_macros)]

mod client;
mod world;

fn main() {
    let mut client = client::Client::new();
    client.run();
}
