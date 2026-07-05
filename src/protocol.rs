use crate::executor::Operation;
use std::fmt::Display;

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

/// Reads a one-line command and returns an Operation enum
pub fn parse_command(command: &str) -> Result<Operation, ParseError> {
    if command.is_empty() {
        return Err(ParseError::Empty);
    }
    let (verb, rest) = command.split_once(' ').unwrap_or((command, ""));

    match verb.to_ascii_uppercase().as_str() {
        "SET" => {
            let (key, value_str) = rest.split_once(' ').ok_or(ParseError::MissingArguments)?;
            Ok(Operation::Set(
                String::from(key),
                value_str.as_bytes().to_vec(),
            ))
        }
        "GET" => {
            let key = String::from(rest);
            Ok(Operation::Get(key))
        }
        "DELETE" => {
            let key = String::from(rest);
            Ok(Operation::Delete(key))
        }
        "PING" => Ok(Operation::Ping),
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

        let Operation::Set(k, v) = parse_command(command).unwrap() else {
            panic!("{:?}", command);
        };
        assert_eq!(k, "foo");
        assert_eq!(v, b"hello world".to_vec());
    }

    #[test]
    fn parse_get_command() {
        let command = "GET foo";
        let Operation::Get(k) = parse_command(command).unwrap() else {
            panic!("{:?}", command);
        };
        assert_eq!(k, "foo");
    }

    #[test]
    fn parse_delete_command() {
        let command = "DELETE foo";
        let Operation::Delete(k) = parse_command(command).unwrap() else {
            panic!("{:?}", command);
        };
        assert_eq!(k, "foo");
    }
}
