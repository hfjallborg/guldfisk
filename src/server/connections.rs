use crate::executor::{Instruction, InstructionSendError, Response};
use crate::protocol::parse_command;
use crossbeam_channel::Sender;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::{env, thread};

pub trait Listener {
    type Stream: Read + Write + Send + 'static;
    fn accept_connection(&self, sender: Sender<Instruction>) -> std::io::Result<Self::Stream>;
}

impl Listener for TcpListener {
    type Stream = TcpStream;
    fn accept_connection(&self, _sender: Sender<Instruction>) -> std::io::Result<TcpStream> {
        let stream = self.accept()?.0;
        Ok(stream)
    }
}

impl Listener for UnixListener {
    type Stream = UnixStream;
    fn accept_connection(&self, _sender: Sender<Instruction>) -> std::io::Result<UnixStream> {
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
    let port = env::var("GULDFISK_PORT").unwrap_or_else(|_| "1983".to_string());
    format!("127.0.0.1:{}", port)
}

/// Loops through a stream, reading and handling commands
fn accept_commands<S: Read + Write>(
    mut stream: S,
    sender: Sender<Instruction>,
) -> Result<(), std::io::Error> {
    let mut buf = BufReader::new(&mut stream);
    let mut line = String::new();
    loop {
        line.clear();
        if buf.read_line(&mut line)? == 0 {
            return Ok(());
        }
        let (_rs, _rr) = oneshot::channel::<Response>();

        let op = match parse_command(line.trim_end_matches(['\n', '\r'])) {
            Ok(op) => op,
            Err(e) => {
                write!(buf.get_mut(), "ERR {}\r\n", e)?;
                continue;
            }
        };

        match Instruction::send(op, sender.clone()) {
            Ok(Response::Return(value)) => {
                buf.get_mut().write_all(&value)?;
                buf.get_mut().write_all(b"\r\n")?;
            }
            Ok(Response::Ok()) => {
                buf.get_mut().write_all(b"OK\r\n")?;
            }
            Ok(Response::Error(kind)) => {
                write!(buf.get_mut(), "ERR {}\r\n", kind)?;
            }
            Err(InstructionSendError::Send(e)) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("Failed to send instruction: {}", e),
                ));
            }
            Err(InstructionSendError::Recv(e)) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("Failed to receive response: {}", e),
                ));
            }
        }
    }
}

/// Spawns a thread to handle a single connection
fn handle_connection<S: Read + Write + Send + 'static>(
    stream: S,
    sender: Sender<Instruction>,
) -> std::io::Result<()> {
    thread::spawn(move || -> std::io::Result<()> {
        accept_commands(stream, sender)?;
        Ok(())
    });

    Ok(())
}

/// Accepts connections from a listener (Unix or TCP) and passes each new stream to
/// the connection handler
pub fn accept_connections<L: Listener>(
    listener: L,
    sender: Sender<Instruction>,
) -> std::io::Result<()> {
    loop {
        let stream = listener.accept_connection(sender.clone())?;
        handle_connection(stream, sender.clone())?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::executor::run;
    use crate::expiration::ExpirationTable;
    use crossbeam_channel::unbounded;
    use std::io::{BufRead, BufReader, BufWriter};
    use std::thread;

    #[test]
    fn test_accept_tcp_connections() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (s, r) = unbounded();
        thread::spawn(move || run(r, Cache::new(), ExpirationTable::new()));
        thread::spawn(move || accept_connections(listener, s));

        let connection = TcpStream::connect(addr).unwrap();
        let mut reader = BufReader::new(connection.try_clone().unwrap());
        let mut writer = BufWriter::new(connection);
        let mut buffer = String::new();
        writer.write_all(b"PING\r\n").unwrap();
        writer.flush().unwrap();
        reader.read_line(&mut buffer).unwrap();
        assert_eq!(buffer, "OK\r\n");
    }

    #[test]
    fn test_accept_unix_connections() {
        let path = env::temp_dir().join(format!(
            "guldfisk-test-{}-{}.sock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        let listener = UnixListener::bind(&path).unwrap();

        let (s, r) = unbounded();
        thread::spawn(move || run(r, Cache::new(), ExpirationTable::new()));
        thread::spawn(move || accept_connections(listener, s));

        let connection = UnixStream::connect(&path).unwrap();

        let mut reader = BufReader::new(connection.try_clone().unwrap());
        let mut writer = BufWriter::new(connection);
        let mut buffer = String::new();
        writer.write_all(b"PING\r\n").unwrap();
        writer.flush().unwrap();
        reader.read_line(&mut buffer).unwrap();
        assert_eq!(buffer, "OK\r\n");

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_recover_after_parse_error() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (s, r) = unbounded();
        thread::spawn(move || run(r, Cache::new(), ExpirationTable::new()));
        thread::spawn(move || accept_connections(listener, s));

        let connection = TcpStream::connect(addr).unwrap();
        let mut reader = BufReader::new(connection.try_clone().unwrap());
        let mut writer = BufWriter::new(connection);
        let mut buffer = String::new();

        // Invalid command
        writer.write_all(b"INVALID\r\n").unwrap();
        writer.flush().unwrap();
        reader.read_line(&mut buffer).unwrap();

        assert!(buffer.starts_with("ERR"));
        assert!(buffer.ends_with("\r\n"));

        buffer.clear();

        // Valid command
        writer.write_all(b"PING\r\n").unwrap();
        writer.flush().unwrap();
        reader.read_line(&mut buffer).unwrap();

        assert_eq!(buffer, "OK\r\n");
    }
}
