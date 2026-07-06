use crate::cache::Cache;
use crate::expiration::ExpirationTable;
use crossbeam_channel::{Receiver, SendError};
use std::fmt::Display;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub enum Operation {
    Set(String, Vec<u8>),
    Get(String),
    Delete(String),
    Expire(String, Duration),
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
    pub timestamp: SystemTime,
}

pub enum InstructionSendError {
    Send(SendError<Instruction>),
    Recv(oneshot::RecvError),
}

impl From<SendError<Instruction>> for InstructionSendError {
    fn from(err: SendError<Instruction>) -> Self {
        InstructionSendError::Send(err)
    }
}

impl From<oneshot::RecvError> for InstructionSendError {
    fn from(err: oneshot::RecvError) -> Self {
        InstructionSendError::Recv(err)
    }
}

impl Instruction {
    /// Sends an instruction to the executor and waits for a response.
    /// Sets the necessary fields in the instruction struct and creates the oneshot channel for the response.
    pub fn send(
        op: Operation,
        sender: crossbeam_channel::Sender<Instruction>,
    ) -> Result<Response, InstructionSendError> {
        let (rs, rr) = oneshot::channel::<Response>();
        let instruction = Instruction {
            op,
            timestamp: SystemTime::now(),
            reply: rs,
        };
        sender.send(instruction)?;
        let response = rr.recv()?;
        Ok(response)
    }
}

pub fn run(
    receiver: Receiver<Instruction>,
    mut cache: Cache,
    mut expiration_table: ExpirationTable,
) {
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
                        // check expiry
                        if expiration_table.is_expired(&key, instruction.timestamp) {
                            cache.delete(&key);
                            expiration_table.delete(&key);
                            instruction
                                .reply
                                .send(Response::Error(ErrorKind::KeyNotFound))
                                .unwrap();
                        } else {
                            instruction.reply.send(Response::Return(value)).unwrap();
                        }
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
            Operation::Expire(key, ttl) => {
                expiration_table.add(key, ttl);
                instruction.reply.send(Response::Ok()).unwrap();
            }
            Operation::Ping => {
                instruction.reply.send(Response::Ok()).unwrap();
            }
        }
    }
}
