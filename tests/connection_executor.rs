use crossbeam_channel::{Sender, unbounded};
use guldfisk::cache::Cache;
use guldfisk::executor::{ErrorKind, Instruction, Operation, Response, run};
use guldfisk::expiration::ExpirationTable;
use std::thread;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

fn create_cache_thread() -> Sender<Instruction> {
    let (s, r) = unbounded::<Instruction>();

    let cache = Cache::new();
    thread::spawn(move || {
        run(r, cache, ExpirationTable::new());
    });
    s
}

fn execute(s: &Sender<Instruction>, op: Operation) -> Response {
    let (rs, rr) = oneshot::channel::<Response>();
    s.send(Instruction {
        op,
        reply: rs,
        timestamp: SystemTime::now(),
    })
    .unwrap();
    rr.recv().unwrap()
}

#[test]
fn set_returns_ok() {
    let s = create_cache_thread();

    let res = execute(&s, Operation::Set("Foo".to_string(), b"Bar".to_vec()));
    assert!(matches!(res, Response::Ok()));
}

#[test]
fn set_then_get_returns_result() {
    let s = create_cache_thread();
    execute(&s, Operation::Set("Foo".to_string(), b"Bar".to_vec()));
    let res = execute(&s, Operation::Get("Foo".to_string()));
    let Response::Return(value) = res else {
        panic!("Expected Return response");
    };
    assert_eq!(value, b"Bar".to_vec());
}

#[test]
fn double_set_updates_value() {
    let s = create_cache_thread();
    execute(&s, Operation::Set("Foo".to_string(), b"Bar".to_vec()));
    execute(&s, Operation::Set("Foo".to_string(), b"Baz".to_vec()));
    let res = execute(&s, Operation::Get("Foo".to_string()));
    let Response::Return(value) = res else {
        panic!("Expected Return response");
    };
    assert_eq!(value, b"Baz".to_vec());
}

#[test]
fn get_non_existing_key_returns_error() {
    let s = create_cache_thread();
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
    let s = create_cache_thread();
    let res = execute(&s, Operation::Delete("Foo".to_string()));
    assert!(matches!(res, Response::Ok()));
}

#[test]
fn delete_existing_key_then_get_returns_error() {
    let s = create_cache_thread();
    execute(&s, Operation::Set("Foo".to_string(), b"Bar".to_vec()));
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
    let s = create_cache_thread();

    match Instruction::send(
        Operation::Set("Foo".to_string(), b"Bar".to_vec()),
        s.clone(),
    ) {
        Ok(_) => {}
        Err(_) => panic!("Expected Ok response"),
    }
    match Instruction::send(
        Operation::Expire("Foo".to_string(), Duration::from_secs(2)),
        s.clone(),
    ) {
        Ok(_) => {}
        Err(_) => panic!("Expected Ok response"),
    }

    // attempt GET before expiration
    match Instruction::send(Operation::Get("Foo".to_string()), s.clone()) {
        Ok(response) => {
            let Response::Return(value) = response else {
                panic!("Expected Return response");
            };
            assert_eq!(value, b"Bar".to_vec());
        }
        Err(_) => panic!("Expected Ok response"),
    }

    sleep(Duration::from_secs(2));

    match Instruction::send(Operation::Get("Foo".to_string()), s.clone()) {
        Ok(response) => {
            let Response::Error(ErrorKind::KeyNotFound) = response else {
                panic!("Expected Error response");
            };
            assert!(matches!(response, Response::Error(ErrorKind::KeyNotFound)));
        }
        Err(_) => panic!("Expected Ok response"),
    }
}
