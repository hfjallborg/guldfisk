use crate::cache::CacheItem;
use crate::messaging::Message;
use std::fmt::Display;
use std::io::Write;
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Empty,
    MissingArguments,
    UnknownVerb(String),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Empty => write!(f, "Empty command"),
            ParseError::MissingArguments => write!(f, "Missing arguments"),
            ParseError::UnknownVerb(verb) => write!(f, "Unknown verb: {}", verb),
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Set(String, String),
    Get(String),
    Delete(String),
    Expire(String, Duration),
    Subscribe(String),
    Unsubscribe(String),
    Exit,
    Publish(String, String),
    Ping,
}

pub fn format_response(data: &CacheItem) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // First byte will indicate data type, as a one-byte ascii symbol
    match data {
        CacheItem::String(s) => {
            let buf = format_str(s.to_string())?;

            Ok(buf.to_vec())
        }
        CacheItem::Array(len, s) => format_array(*len, s),
    }
}

fn format_str(s: String) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let len = s.len();
    let len_str = len.to_string();
    let mut buf = Vec::with_capacity(len);
    buf.write_all(b"!")?;
    buf.write_all(len_str.as_bytes())?;
    buf.write_all(b"\r\n")?;
    buf.write_all(s.as_bytes())?;
    buf.write_all(b"\r\n")?;
    Ok(buf.to_vec())
}

fn format_array(len: usize, items: &Vec<CacheItem>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut buf: Vec<u8> = Vec::new();

    let len_str = len.to_string();

    buf.write_all(b"#")?;
    buf.write_all(len_str.as_bytes())?;
    buf.write_all(b"\r\n")?;
    for item in items {
        match item {
            CacheItem::String(s) => {
                buf.write_all(format_str(s.clone())?.as_slice())?;
            }
            CacheItem::Array(len, items) => {
                buf.write_all(format_array(*len, items)?.as_slice())?;
            }
        }
    }
    // We do not need to write a trailing \r\n because the last array item will do that
    Ok(buf.to_vec())
}

pub fn format_message(msg: Message) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut buf = Box::new(Vec::with_capacity(
        1 + msg.channel.len() + msg.content.len() + 4,
    ));

    buf.write_all(b"@")?;
    buf.write_all(msg.channel.as_bytes())?;
    buf.write_all(b"\r\n")?;
    let data = format_str(msg.content)?;
    buf.write_all(data.as_slice())?;

    Ok(buf.to_vec())
}

fn parse_str_value(s: &str) -> Result<String, ParseError> {
    let mut buf = String::new();
    let mut quoted = false;
    for c in s.chars() {
        if c == '"' {
            if !quoted {
                quoted = true;
                continue;
            } else {
                break;
            }
        }
        if quoted {
            buf.push(c);
        }
    }
    if !quoted {
        return Err(ParseError::MissingArguments);
    }

    Ok(buf)
}

/// Reads a one-line command and returns an Operation enum
pub fn parse_command(command: &str) -> Result<Command, ParseError> {
    if command.is_empty() {
        return Err(ParseError::Empty);
    }
    let (verb, rest) = command.split_once(' ').unwrap_or((command, ""));

    match verb.to_ascii_uppercase().as_str() {
        "SET" => {
            let (key, value_str) = rest.split_once(' ').ok_or(ParseError::MissingArguments)?;

            let value = parse_str_value(value_str)?;

            Ok(Command::Set(String::from(key), value))
        }
        "GET" => {
            let key = String::from(rest);
            Ok(Command::Get(key))
        }
        "DELETE" => {
            let key = String::from(rest);
            Ok(Command::Delete(key))
        }
        "EXPIRE" => {
            let (key, duration_str) = rest.split_once(' ').ok_or(ParseError::MissingArguments)?;
            let duration = duration_str
                .parse::<u64>()
                .map_err(|_| ParseError::MissingArguments)?;
            Ok(Command::Expire(
                String::from(key),
                Duration::from_secs(duration),
            ))
        }
        "SUBSCRIBE" => {
            let channel = String::from(rest);
            Ok(Command::Subscribe(channel))
        }
        "UNSUBSCRIBE" => {
            let channel = String::from(rest);
            Ok(Command::Unsubscribe(channel))
        }
        "PUBLISH" => {
            let (channel, content) = rest.split_once(' ').ok_or(ParseError::MissingArguments)?;
            let content = parse_str_value(content)?;
            Ok(Command::Publish(String::from(channel), content))
        }
        "PING" => Ok(Command::Ping),
        "EXIT" => Ok(Command::Exit),
        _ => Err(ParseError::UnknownVerb(verb.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    #[test]
    fn empty_command_returns_err() {
        let command = "";
        let res = parse_command(command);
        assert!(res.is_err());
        assert_matches!(res, Err(ParseError::Empty));
    }

    #[test]
    fn missing_arguments_returns_err() {
        let mut command = "SET foo";
        let mut res = parse_command(command);
        assert!(res.is_err());
        assert_matches!(res, Err(ParseError::MissingArguments));

        command = "SET";
        res = parse_command(command);
        assert!(res.is_err());
        assert_matches!(res, Err(ParseError::MissingArguments));
    }

    #[test]
    fn invalid_verb_returns_err() {
        let command = "HELLO world";
        let res = parse_command(command);
        assert!(res.is_err());
        assert_matches!(res, Err(ParseError::UnknownVerb(_)));
    }

    #[test]
    fn parse_set_command() {
        let command = "SET foo \"hello world\"";

        let Command::Set(k, v) = parse_command(command).unwrap() else {
            panic!("{:?}", command);
        };
        assert_eq!(k, "foo");
        assert_eq!(v, "hello world");
    }

    #[test]
    fn parse_set_command_unquoted_returns_err() {
        let command = "SET foo hello world";
        let res = parse_command(command);
        assert!(res.is_err());
        assert_matches!(res, Err(ParseError::MissingArguments));
    }

    #[test]
    fn parse_publish_command() {
        let command = "PUBLISH foo \"hello world\"";

        let Command::Publish(channel, content) = parse_command(command).unwrap() else {
            panic!("{:?}", command);
        };
        assert_eq!(channel, "foo");
        assert_eq!(content, "hello world");
    }

    #[test]
    fn parse_publish_command_unquoted_returns_err() {
        let command = "PUBLISH foo hello world";
        let res = parse_command(command);
        assert!(res.is_err());
        assert_matches!(res, Err(ParseError::MissingArguments));
    }

    #[test]
    fn parse_get_command() {
        let command = "GET foo";
        let Command::Get(k) = parse_command(command).unwrap() else {
            panic!("{:?}", command);
        };
        assert_eq!(k, "foo");
    }

    #[test]
    fn parse_delete_command() {
        let command = "DELETE foo";
        let Command::Delete(k) = parse_command(command).unwrap() else {
            panic!("{:?}", command);
        };
        assert_eq!(k, "foo");
    }

    #[test]
    fn test_format_str_response() {
        let data = CacheItem::String(String::from("hello world"));
        let response = format_response(&data).unwrap();
        assert_eq!(response, b"!11\r\nhello world\r\n");
    }

    #[test]
    fn test_format_message() {
        let msg = Message {
            content: String::from("hello world"),
            channel: String::from("news"),
            timestamp: std::time::SystemTime::now(),
        };
        let response = format_message(msg).unwrap();
        assert_eq!(response, b"@news\r\n!11\r\nhello world\r\n");
    }

    #[test]
    fn test_format_array_response() {
        let data = CacheItem::Array(
            2,
            vec![
                CacheItem::String(String::from("hello")),
                CacheItem::String(String::from("world")),
            ],
        );
        let response = format_response(&data).unwrap();
        assert_eq!(response, b"#2\r\n!5\r\nhello\r\n!5\r\nworld\r\n");
    }
}
