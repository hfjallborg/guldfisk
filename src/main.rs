use crossbeam_channel::unbounded;
use guldfisk::cache::Cache;
use guldfisk::executor::{self, Instruction};
use guldfisk::expiration::ExpirationTable;
use guldfisk::messaging::SubscriptionTable;
use guldfisk::server::connections::{accept_connections, bind_unix_socket, create_addr};
use std::net::TcpListener;
use std::path::Path;
use std::thread;
use std::time::Duration;

const ACTIVE_EXPIRATION_INTERVAL: Duration = Duration::from_millis(1000);

fn main() -> std::io::Result<()> {
    let addr = create_addr();

    let (s, r) = unbounded::<Instruction>();

    let cache = Cache::new();
    let expiration_table = ExpirationTable::new();
    thread::spawn(move || {
        executor::run(
            r,
            cache,
            expiration_table,
            SubscriptionTable::new(),
            ACTIVE_EXPIRATION_INTERVAL,
        )
    });

    let tcp_listener = TcpListener::bind(&addr)?;
    let unix_listener = bind_unix_socket(Path::new("/tmp/guldfisk.sock"))?;
    let unix_sender = s.clone();

    thread::spawn(|| {
        println!("Listening for Unix sockets on /tmp/guldfisk.sock");
        accept_connections(unix_listener, unix_sender)
    });

    println!("Listening for TCP connections on {}", addr);
    accept_connections(tcp_listener, s)
}
