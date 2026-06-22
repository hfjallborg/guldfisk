use std::env;
use std::io::Write;
use std::net::{TcpListener, TcpStream};

pub fn create_addr() -> String {
    // Creates the TCP listener address from env variables
    let port = env::var("IRIS_PORT").unwrap_or_else(|_| "1983".to_string());
    format!("127.0.0.1:{}", port)
}

pub fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    // Handles a new connection, given as a TCP stream
    // Currently only returns an empty 501
    stream.write_all(b"HTTP/1.1 501 Not Implemented\r\n\r\n")?;
    stream.flush()
}

pub fn accept_connections(tcp_listener: TcpListener) -> std::io::Result<()> {
    for stream in tcp_listener.incoming() {
        handle_connection(stream?)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::thread;

    #[test]
    fn test_accept_connections() {
        // Bind to port 0 so the OS assigns a free port, then read it back.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || accept_connections(listener));

        let connection = TcpStream::connect(addr).unwrap();
        let mut line = String::new();
        BufReader::new(connection).read_line(&mut line).unwrap();
        assert_eq!(line, "HTTP/1.1 501 Not Implemented\r\n");
    }
}
