use crate::cache::Cache;
use crossbeam_channel::Receiver;

#[derive(Debug)]
pub enum Operation {
    Set(String, Vec<u8>),
    Get(String),
    Delete(String),
}

pub enum ErrorKind {
    KeyNotFound,
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
                println!("set: {} to {}", key, String::from_utf8_lossy(&value));
                cache.set(&key, value);
                instruction.reply.send(Response::Ok()).unwrap();
            }
            Operation::Get(key) => {
                println!("get: {}", key);
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
                println!("delete: {}", key);
                cache.delete(&key);
                instruction.reply.send(Response::Ok()).unwrap();
            }
        }
    }
}
