use crate::executor::{Instruction, InstructionSendError, Operation, Response};
use crate::messaging::Message;
use crate::protocol::{Command, format_message, parse_command};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::thread::JoinHandle;
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

pub trait TryCloneStream: Sized {
    fn try_clone(&self) -> std::io::Result<Self>;
}

impl TryCloneStream for TcpStream {
    fn try_clone(&self) -> std::io::Result<Self> {
        self.try_clone()
    }
}

impl TryCloneStream for UnixStream {
    fn try_clone(&self) -> std::io::Result<Self> {
        self.try_clone()
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

pub fn receive_messages<S: Read + Write + Send + 'static>(
    rx: Receiver<Message>,
    mut buf_writer: BufWriter<S>,
) -> std::io::Result<()> {
    rx.iter().try_for_each(|msg| {
        buf_writer.write_all(format_message(msg).unwrap().as_slice())?;
        buf_writer.flush()?;
        Ok(())
    })
}

/// Loops through a stream, reading and handling commands
fn accept_commands<S: Read + Write + TryCloneStream + Send + 'static>(
    stream: S,
    sender: &Sender<Instruction>,
    connection_id: u64,
) -> Result<Option<JoinHandle<Result<(), std::io::Error>>>, std::io::Error> {
    let mut buf = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    let (msg_tx, msg_rx) = unbounded::<Message>();
    let mut msg_rx = Some(msg_rx);
    let mut msg_handle: Option<JoinHandle<Result<(), std::io::Error>>> = None;
    loop {
        line.clear();
        if buf.read_line(&mut line)? == 0 {
            return Ok(msg_handle);
        }

        let cmd = match parse_command(line.trim_end_matches(['\n', '\r'])) {
            Ok(cmd) => cmd,
            Err(e) => {
                write!(buf.get_mut(), "ERR {}\r\n", e)?;
                continue;
            }
        };

        let op = match cmd {
            Command::Set(key, item) => Operation::Set(key, item),
            Command::Get(key) => Operation::Get(key),
            Command::Delete(key) => Operation::Delete(key),
            Command::Expire(key, ttl) => Operation::Expire(key, ttl),
            Command::Subscribe(channel) => {
                if let Some(rx) = msg_rx.take() {
                    let write_half = stream.try_clone()?;
                    msg_handle = Some(thread::spawn(move || {
                        receive_messages(rx, BufWriter::new(write_half))
                    }));
                }
                Operation::Subscribe(channel, msg_tx.clone())
            }
            Command::Unsubscribe(channel) => Operation::Unsubscribe(channel),
            Command::Publish(channel, content) => Operation::Publish(channel, content),
            Command::Ping => Operation::Ping,
            Command::Exit => Operation::Terminate,
        };
        let is_exit = matches!(op, Operation::Terminate);

        match Instruction::send(op, sender.clone(), connection_id) {
            Ok(Response::Return(value)) => {
                buf.get_mut().write_all(&value)?;
                buf.get_mut().write_all(b"\r\n")?;
            }
            Ok(Response::Ok()) => {
                buf.get_mut().write_all(b"OK\r\n")?;
                if is_exit {
                    return Ok(msg_handle);
                }
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

/// "Gracefully" quit the connection
///
/// Sends an instruction to remove all subscriptions
fn terminate_connection(sender: Sender<Instruction>, connection_id: u64) {
    match Instruction::send(Operation::Terminate, sender.clone(), connection_id) {
        Ok(Response::Ok()) => {}
        Err(InstructionSendError::Send(e)) => {
            println!("Failed to send terminate instruction: {}", e);
        }
        Err(InstructionSendError::Recv(e)) => {
            println!("Failed to receive terminate response: {}", e);
        }
        _ => {
            println!("Unexpected response when terminating connection");
        }
    }
}

/// Spawns a thread to handle a single connection
fn handle_connection<S: Read + Write + Send + TryCloneStream + 'static>(
    stream: S,
    sender: Sender<Instruction>,
    id: u64,
) -> std::io::Result<()> {
    thread::spawn(move || -> std::io::Result<()> {
        let msg_handle = accept_commands(stream, &sender, id)?;
        terminate_connection(sender, id);
        if let Some(handle) = msg_handle {
            match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => println!("Message receiver thread exited with error: {}", e),
                Err(_) => println!("Message receiver thread panicked"),
            }
        }
        Ok(())
    });

    Ok(())
}

/// Accepts connections from a listener (Unix or TCP) and passes each new stream to
/// the connection handler
pub fn accept_connections<L: Listener>(
    listener: L,
    sender: Sender<Instruction>,
) -> std::io::Result<()>
where
    <L as Listener>::Stream: TryCloneStream,
{
    let mut counter: u64 = 0;

    loop {
        let stream = listener.accept_connection(sender.clone())?;
        handle_connection(stream, sender.clone(), counter)?;
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::executor::run;
    use crate::expiration::ExpirationTable;
    use crate::messaging::SubscriptionTable;
    use crossbeam_channel::unbounded;
    use std::io::{BufRead, BufReader, BufWriter};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_accept_tcp_connections() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (s, r) = unbounded();
        thread::spawn(move || {
            run(
                r,
                Cache::new(),
                ExpirationTable::new(),
                SubscriptionTable::new(),
                Duration::from_secs(3600),
            )
        });
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
        thread::spawn(move || {
            run(
                r,
                Cache::new(),
                ExpirationTable::new(),
                SubscriptionTable::new(),
                Duration::from_secs(3600),
            )
        });
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
        thread::spawn(move || {
            run(
                r,
                Cache::new(),
                ExpirationTable::new(),
                SubscriptionTable::new(),
                Duration::from_secs(3600),
            )
        });
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
