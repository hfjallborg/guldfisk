use crossbeam_channel::unbounded;
use guldfisk::cache::Cache;
use guldfisk::executor::{Instruction, run};
use guldfisk::expiration::ExpirationTable;
use guldfisk::messaging::SubscriptionTable;
use guldfisk::server::connections::accept_connections;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn main() {
    divan::main()
}

const NUM_KEYS: usize = 100_000;

#[divan::bench(args = [8, 32, 64, 128])]
fn cache_set(b: divan::Bencher, key_len: usize) {
    let mut c = Cache::new();

    let keys: Vec<String> = (0..NUM_KEYS).map(|i| format!("{i:0>key_len$}")).collect();

    let mut i = 0;
    b.with_inputs(|| {
        let key = &keys[i % NUM_KEYS];
        i += 1;
        (key, b"value".to_vec())
    })
    .bench_local_values(|(key, value)| {
        c.set(key, value);
    });
}

#[divan::bench(args = [8, 32, 64, 128])]
fn set_tcp_roundtrip(b: divan::Bencher, key_len: usize) {
    // Execution thread
    let c = Cache::new();
    let (s, r) = unbounded::<Instruction>();
    thread::spawn(move || {
        run(r, c, ExpirationTable::new(), SubscriptionTable::new());
    });

    // TCP thread
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(|| {
        accept_connections(listener, s).unwrap();
    });

    // TCP client
    let connection = TcpStream::connect(addr).unwrap();
    let mut reader = BufReader::new(connection.try_clone().unwrap());
    let mut writer = BufWriter::new(connection);
    let mut buffer = String::new();

    let keys: Vec<String> = (0..NUM_KEYS).map(|i| format!("{i:0>key_len$}")).collect();
    let mut i = 0;
    b.with_inputs(|| {
        let key = &keys[i % NUM_KEYS];
        i += 1;
        (key, b"value".to_vec())
    })
    .bench_local_values(|(key, value)| {
        writer
            .write_all(format!("SET {} {}\r\n", key, String::from_utf8_lossy(&value)).as_bytes())
            .unwrap();
        writer.flush().unwrap();
        buffer.clear();
        reader.read_line(&mut buffer).unwrap();
    })
}
