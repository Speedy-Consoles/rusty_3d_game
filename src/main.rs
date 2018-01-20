#![feature(use_extern_macros)]

mod client;

fn main() {
    let mut client = client::Client::new();
    client.run();
}
