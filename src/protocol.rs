use std::fmt::Display;
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

/// Reads a one-line command and returns an Operation enum
pub fn parse_command(command: &str) -> Result<Command, ParseError> {
    if command.is_empty() {
        return Err(ParseError::Empty);
    }
    let (verb, rest) = command.split_once(' ').unwrap_or((command, ""));

    match verb.to_ascii_uppercase().as_str() {
        "SET" => {
            let (key, value_str) = rest.split_once(' ').ok_or(ParseError::MissingArguments)?;
            Ok(Command::Set(String::from(key), String::from(value_str)))
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
            Ok(Command::Publish(
                String::from(channel),
                String::from(content),
            ))
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
        let command = "SET foo hello world";

        let Command::Set(k, v) = parse_command(command).unwrap() else {
            panic!("{:?}", command);
        };
        assert_eq!(k, "foo");
        assert_eq!(v, "hello world");
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
}
