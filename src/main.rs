use std::net::TcpListener;

use crate::server::connections::{accept_connections, create_addr};

mod server;

fn main() -> std::io::Result<()> {
    let addr = create_addr();
    println!("iRiS listening on {addr}");

    let listener = TcpListener::bind(&addr)?;
    accept_connections(listener)
}
