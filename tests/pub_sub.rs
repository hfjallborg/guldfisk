use crate::utils::{create_execution_thread, create_tcp_listener_thread};
use crossbeam_channel::unbounded;
use guldfisk::cache::CacheItem;
use guldfisk::executor::{Instruction, Operation, Response};
use guldfisk::messaging::Message;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::thread;

mod utils;

#[test]
fn test_simple_pub_sub() {
    let inst_tx = create_execution_thread();

    let (tx, rx) = unbounded::<Message>();
    match Instruction::send(
        Operation::Subscribe("test".to_string(), tx),
        inst_tx.clone(),
        1,
    ) {
        Ok(_) => {}
        Err(_e) => panic!("Failed to send subscribe instruction"),
    }

    match Instruction::send(
        Operation::Publish(
            "test".to_string(),
            CacheItem::String("Hello, World!".to_string()),
        ),
        inst_tx,
        1,
    ) {
        Ok(_) => {}
        Err(_e) => panic!("Failed to send publish instruction"),
    }

    let msg = rx.recv().unwrap();
    assert_eq!(msg.content, CacheItem::String("Hello, World!".to_string()));
}

#[test]
fn test_multiple_channels() {
    let inst_tx = create_execution_thread();

    let (news_tx, news_rx) = unbounded::<Message>();
    let (sports_tx, sports_rx) = unbounded::<Message>();
    Instruction::send(
        Operation::Subscribe("news".to_string(), news_tx),
        inst_tx.clone(),
        1,
    )
    .unwrap_or_else(|_| panic!("Failed to send subscribe instruction"));
    Instruction::send(
        Operation::Subscribe("sports".to_string(), sports_tx),
        inst_tx.clone(),
        1,
    )
    .unwrap_or_else(|_| panic!("Failed to send subscribe instruction"));

    Instruction::send(
        Operation::Publish(
            "news".to_string(),
            CacheItem::String("Breaking!".to_string()),
        ),
        inst_tx.clone(),
        1,
    )
    .unwrap_or_else(|_| panic!("Failed to send publish instruction"));
    Instruction::send(
        Operation::Publish("sports".to_string(), CacheItem::String("Goal!".to_string())),
        inst_tx,
        1,
    )
    .unwrap_or_else(|_| panic!("Failed to send publish instruction"));

    let news_msg = news_rx.recv().unwrap();
    assert_eq!(news_msg.content, CacheItem::String("Breaking!".to_string()));
    assert_eq!(news_msg.channel, "news");

    let sports_msg = sports_rx.recv().unwrap();
    assert_eq!(sports_msg.content, CacheItem::String("Goal!".to_string()));
    assert_eq!(sports_msg.channel, "sports");

    // neither subscriber should have received the other channel's message
    assert!(news_rx.try_recv().is_err());
    assert!(sports_rx.try_recv().is_err());
}

#[test]
fn test_multiple_subscribers() {
    let inst_tx = create_execution_thread();

    let (tx1, rx1) = unbounded::<Message>();
    let (tx2, rx2) = unbounded::<Message>();
    Instruction::send(
        Operation::Subscribe("test".to_string(), tx1),
        inst_tx.clone(),
        1,
    )
    .unwrap_or_else(|_| panic!("Failed to send subscribe instruction"));
    Instruction::send(
        Operation::Subscribe("test".to_string(), tx2),
        inst_tx.clone(),
        2,
    )
    .unwrap_or_else(|_| panic!("Failed to send subscribe instruction"));

    let response = Instruction::send(
        Operation::Publish(
            "test".to_string(),
            CacheItem::String("Hello, World!".to_string()),
        ),
        inst_tx,
        1,
    )
    .unwrap_or_else(|_| panic!("Failed to send publish instruction"));

    assert_eq!(
        rx1.recv().unwrap().content,
        CacheItem::String("Hello, World!".to_string())
    );
    assert_eq!(
        rx2.recv().unwrap().content,
        CacheItem::String("Hello, World!".to_string())
    );

    let Response::Return(count) = response else {
        panic!("Expected Return response");
    };
    assert_eq!(count, b"2".to_vec());
}

#[test]
fn test_terminate_removes_all_subscriptions_for_connection() {
    let inst_tx = create_execution_thread();

    let (news_tx, news_rx) = unbounded::<Message>();
    let (sports_tx, sports_rx) = unbounded::<Message>();
    Instruction::send(
        Operation::Subscribe("news".to_string(), news_tx),
        inst_tx.clone(),
        1,
    )
    .unwrap_or_else(|_| panic!("Failed to send subscribe instruction"));
    Instruction::send(
        Operation::Subscribe("sports".to_string(), sports_tx),
        inst_tx.clone(),
        1,
    )
    .unwrap_or_else(|_| panic!("Failed to send subscribe instruction"));

    Instruction::send(Operation::Terminate, inst_tx.clone(), 1)
        .unwrap_or_else(|_| panic!("Failed to send terminate instruction"));

    Instruction::send(
        Operation::Publish(
            "news".to_string(),
            CacheItem::String("Breaking!".to_string()),
        ),
        inst_tx.clone(),
        2,
    )
    .unwrap_or_else(|_| panic!("Failed to send publish instruction"));
    Instruction::send(
        Operation::Publish("sports".to_string(), CacheItem::String("Goal!".to_string())),
        inst_tx,
        2,
    )
    .unwrap_or_else(|_| panic!("Failed to send publish instruction"));

    // both subscriptions belonged to connection 1, so terminating it should leave no subscribers
    assert!(news_rx.try_recv().is_err());
    assert!(sports_rx.try_recv().is_err());
}

#[test]
fn test_unsubscribe_stops_receiving_from_that_channel() {
    let inst_tx = create_execution_thread();

    let (news_tx, news_rx) = unbounded::<Message>();
    let (sports_tx, sports_rx) = unbounded::<Message>();
    Instruction::send(
        Operation::Subscribe("news".to_string(), news_tx),
        inst_tx.clone(),
        1,
    )
    .unwrap_or_else(|_| panic!("Failed to send subscribe instruction"));
    Instruction::send(
        Operation::Subscribe("sports".to_string(), sports_tx),
        inst_tx.clone(),
        1,
    )
    .unwrap_or_else(|_| panic!("Failed to send subscribe instruction"));

    Instruction::send(
        Operation::Unsubscribe("news".to_string()),
        inst_tx.clone(),
        1,
    )
    .unwrap_or_else(|_| panic!("Failed to send unsubscribe instruction"));

    Instruction::send(
        Operation::Publish(
            "news".to_string(),
            CacheItem::String("Breaking!".to_string()),
        ),
        inst_tx.clone(),
        2,
    )
    .unwrap_or_else(|_| panic!("Failed to send publish instruction"));
    Instruction::send(
        Operation::Publish("sports".to_string(), CacheItem::String("Goal!".to_string())),
        inst_tx,
        2,
    )
    .unwrap_or_else(|_| panic!("Failed to send publish instruction"));

    // unsubscribed from "news", so no message should arrive there...
    assert!(news_rx.try_recv().is_err());
    // ...but the "sports" subscription should be unaffected
    assert_eq!(
        sports_rx.recv().unwrap().content,
        CacheItem::String("Goal!".to_string())
    );
}

#[test]
fn test_subscribe_publish_unsubscribe_tcp() {
    let inst_tx = create_execution_thread();
    let addr = create_tcp_listener_thread(inst_tx.clone(), "127.0.0.1:0");

    let connection = TcpStream::connect(addr).unwrap();

    let mut reader = BufReader::new(connection.try_clone().unwrap());
    let mut writer = BufWriter::new(connection);
    let mut buffer = String::new();
    writer.write_all(b"SUBSCRIBE foo\r\n").unwrap();
    writer.flush().unwrap();
    reader.read_line(&mut buffer).unwrap();
    assert_eq!(buffer, "OK\r\n");
    buffer.clear();

    let handle = thread::spawn(move || {
        let connection = TcpStream::connect(addr).unwrap();
        let mut reader = BufReader::new(connection.try_clone().unwrap());
        let mut writer = BufWriter::new(connection);
        let mut buffer = String::new();
        writer.write_all(b"PUBLISH foo \"hello\"\r\n").unwrap();
        writer.flush().unwrap();
        reader.read_line(&mut buffer).unwrap();
        assert_eq!(buffer, "1\r\n");
    });

    handle.join().unwrap();
    reader.read_line(&mut buffer).unwrap();
    assert_eq!(buffer, "@foo\r\n");

    buffer.clear();
    reader.read_line(&mut buffer).unwrap();
    assert_eq!(buffer, "!5\r\n");

    buffer.clear();
    reader.read_line(&mut buffer).unwrap();
    assert_eq!(buffer, "hello\r\n");

    buffer.clear();

    writer.write_all(b"UNSUBSCRIBE foo\r\n").unwrap();
    writer.flush().unwrap();
    reader.read_line(&mut buffer).unwrap();
    assert_eq!(buffer, "OK\r\n");
    buffer.clear();

    let handle = thread::spawn(move || {
        let connection = TcpStream::connect(addr).unwrap();
        let mut reader = BufReader::new(connection.try_clone().unwrap());
        let mut writer = BufWriter::new(connection);
        let mut buffer = String::new();
        writer.write_all(b"PUBLISH foo \"hello\"\r\n").unwrap();
        writer.flush().unwrap();
        reader.read_line(&mut buffer).unwrap();
        assert_eq!(buffer, "0\r\n");
    });

    handle.join().unwrap();
}

#[test]
fn test_subscriber_receives_several_messages_tcp() {
    let inst_tx = create_execution_thread();
    let addr = create_tcp_listener_thread(inst_tx.clone(), "127.0.0.1:0");

    let connection = TcpStream::connect(addr).unwrap();

    let mut reader = BufReader::new(connection.try_clone().unwrap());
    let mut writer = BufWriter::new(connection);
    let mut buffer = String::new();
    writer.write_all(b"SUBSCRIBE foo\r\n").unwrap();
    writer.flush().unwrap();
    reader.read_line(&mut buffer).unwrap();
    assert_eq!(buffer, "OK\r\n");
    buffer.clear();

    for content in ["one", "two", "three"] {
        let handle = thread::spawn(move || {
            let connection = TcpStream::connect(addr).unwrap();
            let mut reader = BufReader::new(connection.try_clone().unwrap());
            let mut writer = BufWriter::new(connection);
            let mut buffer = String::new();
            writer
                .write_all(format!("PUBLISH foo \"{}\"\r\n", content).as_bytes())
                .unwrap();
            writer.flush().unwrap();
            reader.read_line(&mut buffer).unwrap();
            assert_eq!(buffer, "1\r\n");
        });
        handle.join().unwrap();

        buffer.clear();
        reader.read_line(&mut buffer).unwrap();
        assert_eq!(buffer, "@foo\r\n");

        buffer.clear();
        reader.read_line(&mut buffer).unwrap();
        assert_eq!(buffer, format!("!{}\r\n", content.len()));

        buffer.clear();
        reader.read_line(&mut buffer).unwrap();
        assert_eq!(buffer, format!("{}\r\n", content));

        buffer.clear();
    }
}

#[test]
fn test_disconnect_without_unsubscribe_or_exit_cleans_up_subscription() {
    let inst_tx = create_execution_thread();
    let addr = create_tcp_listener_thread(inst_tx.clone(), "127.0.0.1:0");

    {
        let connection = TcpStream::connect(addr).unwrap();
        let mut reader = BufReader::new(connection.try_clone().unwrap());
        let mut writer = BufWriter::new(connection);
        let mut buffer = String::new();
        writer.write_all(b"SUBSCRIBE foo\r\n").unwrap();
        writer.flush().unwrap();
        reader.read_line(&mut buffer).unwrap();
        assert_eq!(buffer, "OK\r\n");
    }

    let mut buffer = String::new();
    for _ in 0..50 {
        let connection = TcpStream::connect(addr).unwrap();
        let mut reader = BufReader::new(connection.try_clone().unwrap());
        let mut writer = BufWriter::new(connection);
        buffer.clear();
        writer.write_all(b"PUBLISH foo \"hello\"\r\n").unwrap();
        writer.flush().unwrap();
        reader.read_line(&mut buffer).unwrap();
        if buffer == "0\r\n" {
            break;
        }
        thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(buffer, "0\r\n");
}
