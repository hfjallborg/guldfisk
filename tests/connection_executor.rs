mod utils;
use crossbeam_channel::Sender;
use guldfisk::cache::CacheItem;
use guldfisk::executor::{ErrorKind, Instruction, Operation, Response};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

fn execute(s: &Sender<Instruction>, op: Operation) -> Response {
    let (rs, rr) = oneshot::channel::<Response>();
    s.send(Instruction {
        op,
        reply: rs,
        timestamp: SystemTime::now(),
        connection_id: 1,
    })
    .unwrap();
    rr.recv().unwrap()
}

#[test]
fn set_returns_ok() {
    let s = utils::create_execution_thread();

    let res = execute(
        &s,
        Operation::Set("Foo".to_string(), CacheItem::String("Bar".to_string())),
    );
    assert!(matches!(res, Response::Ok()));
}

#[test]
fn set_then_get_returns_result() {
    let s = utils::create_execution_thread();
    execute(
        &s,
        Operation::Set("Foo".to_string(), CacheItem::String("Bar".to_string())),
    );
    let res = execute(&s, Operation::Get("Foo".to_string()));
    let Response::Return(value) = res else {
        panic!("Expected Return response");
    };
    assert_eq!(value, b"!3\r\nBar\r\n".to_vec());
}

#[test]
fn double_set_updates_value() {
    let s = utils::create_execution_thread();
    execute(
        &s,
        Operation::Set("Foo".to_string(), CacheItem::String("Bar".to_string())),
    );
    execute(
        &s,
        Operation::Set("Foo".to_string(), CacheItem::String("Baz".to_string())),
    );
    let res = execute(&s, Operation::Get("Foo".to_string()));
    let Response::Return(value) = res else {
        panic!("Expected Return response");
    };
    assert_eq!(value, b"!3\r\nBaz\r\n".to_vec());
}

#[test]
fn get_non_existing_key_returns_error() {
    let s = utils::create_execution_thread();
    let res = execute(&s, Operation::Get("Foo".to_string()));
    match res {
        Response::Error(kind) => {
            assert!(matches!(kind, ErrorKind::KeyNotFound));
        }
        _ => panic!("Expected Error response"),
    }
}

#[test]
fn delete_non_existing_key_returns_ok() {
    let s = utils::create_execution_thread();
    let res = execute(&s, Operation::Delete("Foo".to_string()));
    assert!(matches!(res, Response::Ok()));
}

#[test]
fn delete_existing_key_then_get_returns_error() {
    let s = utils::create_execution_thread();

    execute(
        &s,
        Operation::Set("Foo".to_string(), CacheItem::String("Bar".to_string())),
    );
    execute(&s, Operation::Delete("Foo".to_string()));
    let res = execute(&s, Operation::Get("Foo".to_string()));
    match res {
        Response::Error(kind) => {
            assert!(matches!(kind, ErrorKind::KeyNotFound));
        }
        _ => panic!("Expected Error response"),
    }
}

#[test]
fn get_expired_key_returns_error() {
    let s = utils::create_execution_thread();

    match Instruction::send(
        Operation::Set("Foo".to_string(), CacheItem::String("Bar".to_string())),
        s.clone(),
        1,
    ) {
        Ok(_) => {}
        Err(_) => panic!("Expected Ok response"),
    }
    match Instruction::send(
        Operation::Expire("Foo".to_string(), Duration::from_secs(2)),
        s.clone(),
        1,
    ) {
        Ok(_) => {}
        Err(_) => panic!("Expected Ok response"),
    }

    // attempt GET before expiration
    match Instruction::send(Operation::Get("Foo".to_string()), s.clone(), 1) {
        Ok(response) => {
            let Response::Return(value) = response else {
                panic!("Expected Return response");
            };
            assert_eq!(value, b"!3\r\nBar\r\n".to_vec());
        }
        Err(_) => panic!("Expected Ok response"),
    }

    sleep(Duration::from_secs(2));

    match Instruction::send(Operation::Get("Foo".to_string()), s.clone(), 1) {
        Ok(response) => {
            let Response::Error(ErrorKind::KeyNotFound) = response else {
                panic!("Expected Error response");
            };
            assert!(matches!(response, Response::Error(ErrorKind::KeyNotFound)));
        }
        Err(_) => panic!("Expected Ok response"),
    }
}
