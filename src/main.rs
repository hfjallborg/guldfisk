use crossbeam_channel::unbounded;
use iris::cache::Cache;
use iris::executor::{self, Instruction};
use iris::server::connections::{accept_connections, bind_unix_socket, create_addr};
use std::net::TcpListener;
use std::path::Path;
use std::thread;

fn main() -> std::io::Result<()> {
    let addr = create_addr();

    let (s, r) = unbounded::<Instruction>();

    let cache = Cache::new();
    thread::spawn(move || executor::run(r, cache));

    let tcp_listener = TcpListener::bind(&addr)?;
    let unix_listener = bind_unix_socket(Path::new("/tmp/iris.sock"))?;
    let unix_sender = s.clone();

    thread::spawn(|| {
        println!("Listening for Unix sockets on /tmp/iris.sock");
        accept_connections(unix_listener, unix_sender)
    });

    println!("Listening for TCP connections on {}", addr);
    accept_connections(tcp_listener, s)
}
