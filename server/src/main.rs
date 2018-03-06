extern crate server;

use server::Server;

fn main() {
    let mut server = Server::new();
    server.run();
}
