use crate::cache::Cache;
use crossbeam_channel::Receiver;
use std::fmt::Display;

#[derive(Debug)]
pub enum Operation {
    Set(String, Vec<u8>),
    Get(String),
    Delete(String),
    Ping,
}

pub enum ErrorKind {
    KeyNotFound,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::KeyNotFound => {
                write!(f, "Key not found")
            }
        }
    }
}

pub enum Response {
    Return(Vec<u8>),
    Error(ErrorKind),
    Ok(),
}

pub struct Instruction {
    pub op: Operation,
    pub reply: oneshot::Sender<Response>,
}

pub fn run(receiver: Receiver<Instruction>, mut cache: Cache) {
    // Receives commands from connection threads and executes them.

    for instruction in receiver.iter() {
        match instruction.op {
            Operation::Set(key, value) => {
                cache.set(&key, value);
                instruction.reply.send(Response::Ok()).unwrap();
            }
            Operation::Get(key) => {
                let value = cache.get(&key);
                match value {
                    Some(value) => {
                        instruction.reply.send(Response::Return(value)).unwrap();
                    }
                    None => {
                        instruction
                            .reply
                            .send(Response::Error(ErrorKind::KeyNotFound))
                            .unwrap();
                    }
                }
            }
            Operation::Delete(key) => {
                cache.delete(&key);
                instruction.reply.send(Response::Ok()).unwrap();
            }
            Operation::Ping => {
                instruction.reply.send(Response::Ok()).unwrap();
            }
        }
    }
}
