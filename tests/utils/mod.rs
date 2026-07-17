use crossbeam_channel::{Sender, unbounded};
use guldfisk::cache::Cache;
use guldfisk::executor::{Instruction, run};
use guldfisk::expiration::ExpirationTable;
use guldfisk::messaging::SubscriptionTable;
use guldfisk::server::connections::accept_connections;
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

pub fn create_execution_thread() -> Sender<Instruction> {
    let (s, r) = unbounded::<Instruction>();

    let cache = Cache::new();
    thread::spawn(move || {
        run(
            r,
            cache,
            ExpirationTable::new(),
            SubscriptionTable::new(),
            Duration::from_secs(3600),
        );
    });
    s
}

#[allow(dead_code)]
pub fn create_tcp_listener_thread(s: Sender<Instruction>, addr: &str) -> std::net::SocketAddr {
    let listener = TcpListener::bind(addr).unwrap();
    let addr = listener.local_addr();

    thread::spawn(move || accept_connections(listener, s));

    addr.unwrap()
}
