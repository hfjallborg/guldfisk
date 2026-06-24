use crossbeam_channel::Sender;
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

pub trait Listener {
    type Stream: Read + Write;
    fn accept_connection(&self, sender: Sender<String>) -> std::io::Result<Self::Stream>;
}

impl Listener for TcpListener {
    type Stream = TcpStream;
    fn accept_connection(&self, _sender: Sender<String>) -> std::io::Result<TcpStream> {
        let stream = self.accept()?.0;
        Ok(stream)
    }
}

impl Listener for UnixListener {
    type Stream = UnixStream;
    fn accept_connection(&self, _sender: Sender<String>) -> std::io::Result<UnixStream> {
        let stream = self.accept()?.0;
        Ok(stream)
    }
}

/// Takes a unix path and returns a UnixListener bound to that path.
///
/// If the file already exists, it will check if it's actively listening, otherwise delete it.
pub fn bind_unix_socket(path: &Path) -> std::io::Result<UnixListener> {
    if path.exists() {
        if UnixStream::connect(path).is_ok() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                format!("unix socket already in use: {}", path.display()),
            ));
        }
        std::fs::remove_file(path)?;
    }
    UnixListener::bind(path)
}

pub fn create_addr() -> String {
    // Creates the TCP listener address from env variables
    let port = env::var("IRIS_PORT").unwrap_or_else(|_| "1983".to_string());
    format!("127.0.0.1:{}", port)
}

fn handle_connection<S: Read + Write>(
    mut stream: S,
    _sender: Sender<String>,
) -> std::io::Result<()> {
    // Handles a new connection, given as a TCP stream
    stream.write_all(b"Lorem ipsum\n")?;
    stream.flush()
}

/// Accepts connections from a listener (Unix or TCP) and passes each new stream to
/// the connection handler
pub fn accept_connections<L: Listener>(listener: L, sender: Sender<String>) -> std::io::Result<()> {
    loop {
        let stream = listener.accept_connection(sender.clone())?;
        handle_connection(stream, sender.clone())?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;
    use std::io::{BufRead, BufReader};
    use std::thread;

    #[test]
    fn test_accept_tcp_connections() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (s, _r) = unbounded();
        thread::spawn(move || accept_connections(listener, s));

        let connection = TcpStream::connect(addr).unwrap();
        let mut line = String::new();
        BufReader::new(connection).read_line(&mut line).unwrap();
        assert_eq!(line, "Lorem ipsum\n");
    }

    #[test]
    fn test_accept_unix_connections() {
        // Unique path in the temp dir so the socket file can't collide with a
        // leftover from a previous run, a parallel test, or a running server.
        let path = std::env::temp_dir().join(format!(
            "iris-test-{}-{}.sock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        let listener = UnixListener::bind(&path).unwrap();

        let (s, _r) = unbounded();
        // spawn connection thread
        thread::spawn(move || accept_connections(listener, s));

        // attempt connection
        let connection = UnixStream::connect(&path).unwrap();
        let mut line = String::new();
        BufReader::new(connection).read_line(&mut line).unwrap();
        assert_eq!(line, "Lorem ipsum\n");

        // The socket file isn't auto-removed; clean it up.
        std::fs::remove_file(&path).unwrap();
    }
}
