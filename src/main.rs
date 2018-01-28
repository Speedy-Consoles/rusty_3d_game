#![feature(use_extern_macros, entry_or_default)]

mod client;
mod model;
mod consts;

fn main() {
    let mut client = client::Client::new();
    client.run();
}
