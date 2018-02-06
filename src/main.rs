#[macro_use]
extern crate glium;
extern crate strum;
#[macro_use]
extern crate strum_macros;

mod client;
mod model;
mod shared;

fn main() {
    let mut client = client::Client::new();
    client.run();
}
