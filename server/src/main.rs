extern crate server;

use server::Server;

fn main() {
    let mut server = Server::new().unwrap();
    server.run();
}
