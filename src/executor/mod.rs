use crate::cache::{Cache, CacheItem};
use crate::executor::Response::Return;
use crate::expiration::ExpirationTable;
use crate::messaging::{Message, SubscriptionTable};
use crate::protocol::format_response;
use crossbeam_channel::{Receiver, RecvTimeoutError, SendError, Sender};
use std::fmt::Display;
use std::time::{Duration, SystemTime};

pub enum Operation {
    Set(String, CacheItem),
    Get(String),
    Delete(String),
    Expire(String, Duration),
    Subscribe(String, Sender<Message>),
    Unsubscribe(String),
    Publish(String, CacheItem),
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

pub(crate) fn execute_instruction(
    cache: &mut Cache,
    expiration_table: &mut ExpirationTable,
    subscription_table: &mut SubscriptionTable,
    op: Operation,
    timestamp: SystemTime,
    connection_id: u64,
) -> Result<Response, ErrorKind> {
    match op {
        Operation::Set(key, value) => {
            cache.set(&key, value);
            Ok(Response::Ok())
        }
        Operation::Get(key) => {
            let value = cache.get(&key);
            match value {
                Some(value) => {
                    // check expiry
                    if expiration_table.is_expired(&key, timestamp) {
                        cache.delete(&key);
                        expiration_table.delete(&key);
                        Ok(Response::Error(ErrorKind::KeyNotFound))
                    } else {
                        let data = format_response(value);

                        Ok(Return(data.unwrap()))
                    }
                }
                None => Ok(Response::Error(ErrorKind::KeyNotFound)),
            }
        }
        Operation::Delete(key) => {
            cache.delete(&key);
            Ok(Response::Ok())
        }
        Operation::Expire(key, ttl) => {
            expiration_table.add(key, ttl);
            Ok(Response::Ok())
        }
        Operation::Ping => Ok(Response::Ok()),
        Operation::Subscribe(channel, tx) => {
            subscription_table.add(&channel, tx, connection_id);
            Ok(Response::Ok())
        }
        Operation::Unsubscribe(channel) => {
            subscription_table.remove(&channel, &connection_id);
            Ok(Response::Ok())
        }
        Operation::Publish(channel, content) => {
            let txs = subscription_table.get(&channel);
            let msg = Message {
                content,
                timestamp,
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
            Ok(Return(count.to_string().into_bytes()))
        }
        Operation::Terminate => {
            subscription_table.remove_all(&connection_id);
            Ok(Response::Ok())
        }
    }
}

pub fn run(
    receiver: Receiver<Instruction>,
    mut cache: Cache,
    mut expiration_table: ExpirationTable,
    mut subscription_table: SubscriptionTable,
    active_expiration_interval: Duration,
) {
    loop {
        match receiver.recv_timeout(active_expiration_interval) {
            Ok(instruction) => {
                let result = execute_instruction(
                    &mut cache,
                    &mut expiration_table,
                    &mut subscription_table,
                    instruction.op,
                    instruction.timestamp,
                    instruction.connection_id,
                );
                match result {
                    Ok(response) => instruction.reply.send(response).unwrap(),
                    Err(err) => instruction.reply.send(Response::Error(err)).unwrap(),
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                // Run an expiration pass instead
                for key in expiration_table.sample_expired(SystemTime::now()) {
                    cache.delete(&key);
                    expiration_table.delete(&key);
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}
