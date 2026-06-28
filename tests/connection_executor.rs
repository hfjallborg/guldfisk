use crossbeam_channel::{Sender, unbounded};
use iris::cache::Cache;
use iris::executor::{Instruction, Operation, Response, run};
use std::thread;

fn create_cache_thread() -> Sender<Instruction> {
    let (s, r) = unbounded::<Instruction>();

    let cache = Cache::new();
    thread::spawn(move || {
        run(r, cache);
    });
    s
}

fn execute(s: &Sender<Instruction>, op: Operation) -> Response {
    let (rs, rr) = oneshot::channel::<Response>();
    s.send(Instruction { op, reply: rs }).unwrap();
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
fn get_non_existing_key_returns_error() {
    let s = create_cache_thread();
    let res = execute(&s, Operation::Get("Foo".to_string()));
    match res {
        Response::Error(kind) => {
            assert!(matches!(kind, iris::executor::ErrorKind::KeyNotFound));
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
            assert!(matches!(kind, iris::executor::ErrorKind::KeyNotFound));
        }
        _ => panic!("Expected Error response"),
    }
}
