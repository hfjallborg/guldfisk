use crate::cache::Cache;
use crate::executor::Response::Return;
use crate::expiration::ExpirationTable;
use crate::messaging::{Message, SubscriptionTable};
use crossbeam_channel::{Receiver, SendError, Sender};
use std::fmt::Display;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub enum Operation {
    Set(String, Vec<u8>),
    Get(String),
    Delete(String),
    Expire(String, Duration),
    Subscribe(String, Sender<Message>),
    Unsubscribe(String),
    Publish(String, String),
    Terminate,
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
    pub connection_id: u64,
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
        sender: Sender<Instruction>,
        connection_id: u64,
    ) -> Result<Response, InstructionSendError> {
        let (rs, rr) = oneshot::channel::<Response>();
        let instruction = Instruction {
            op,
            timestamp: SystemTime::now(),
            reply: rs,
            connection_id,
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
    mut subscription_table: SubscriptionTable,
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
            Operation::Subscribe(channel, tx) => {
                subscription_table.add(&channel, tx, instruction.connection_id);
                instruction.reply.send(Response::Ok()).unwrap();
            }
            Operation::Unsubscribe(channel) => {
                subscription_table.remove(&channel, &instruction.connection_id);
                instruction.reply.send(Response::Ok()).unwrap();
            }
            Operation::Publish(channel, content) => {
                let txs = subscription_table.get(&channel);
                let msg = Message {
                    content,
                    timestamp: instruction.timestamp,
                    channel: channel.to_string(),
                };
                let mut count = 0;

                for (id, tx) in txs {
                    match tx.send(msg.clone()) {
                        Ok(_) => count += 1,
                        Err(e) => {
                            println!("Failed to send message: {:?}", e);
                            subscription_table.remove(&channel, &id);
                        }
                    }
                }
                instruction
                    .reply
                    .send(Return(count.to_string().into_bytes()))
                    .unwrap();
            }
            Operation::Terminate => {
                subscription_table.remove_all(&instruction.connection_id);
                instruction.reply.send(Response::Ok()).unwrap();
            }
        }
    }
}
