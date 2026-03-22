//! Integration tests for client-server communication

use macwinshare_core::protocol::{Message, MessageCodec, PROTOCOL_MAJOR, PROTOCOL_MINOR};
use std::io::Cursor;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

/// Test basic TCP message exchange (simulates handshake without TLS)
#[test]
fn test_tcp_message_exchange() {
    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    
    // Spawn server thread
    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        
        // Read Hello from client
        let hello = MessageCodec::read_from(&mut stream).unwrap();
        match hello {
            Message::Hello { name, major, minor } => {
                assert_eq!(name, "TestClient");
                assert_eq!(major, PROTOCOL_MAJOR);
                assert_eq!(minor, PROTOCOL_MINOR);
            }
            _ => panic!("Expected Hello message"),
        }
        
        // Send HelloBack
        let hello_back = Message::hello_back("TestServer");
        MessageCodec::write_to(&mut stream, &hello_back).unwrap();
        
        // Read ScreenInfo from client
        let screen_info = MessageCodec::read_from(&mut stream).unwrap();
        match screen_info {
            Message::ScreenInfo { width, height, .. } => {
                assert_eq!(width, 1920);
                assert_eq!(height, 1080);
            }
            _ => panic!("Expected ScreenInfo message"),
        }
        
        // Send InfoAck
        MessageCodec::write_to(&mut stream, &Message::InfoAck).unwrap();
    });
    
    // Client side
    let mut client = TcpStream::connect(addr).unwrap();
    client.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    
    // Send Hello
    let hello = Message::hello("TestClient");
    MessageCodec::write_to(&mut client, &hello).unwrap();
    
    // Read HelloBack
    let hello_back = MessageCodec::read_from(&mut client).unwrap();
    match hello_back {
        Message::HelloBack { name, .. } => {
            assert_eq!(name, "TestServer");
        }
        _ => panic!("Expected HelloBack message"),
    }
    
    // Send ScreenInfo
    let screen_info = Message::ScreenInfo {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        cursor_x: 960,
        cursor_y: 540,
    };
    MessageCodec::write_to(&mut client, &screen_info).unwrap();
    
    // Read InfoAck
    let info_ack = MessageCodec::read_from(&mut client).unwrap();
    assert!(matches!(info_ack, Message::InfoAck));
    
    server_handle.join().unwrap();
}

/// Test multiple messages in sequence
#[test]
fn test_message_sequence() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    
    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        
        // Receive multiple mouse move messages
        for expected_x in [100i16, 200, 300, 400, 500] {
            let msg = MessageCodec::read_from(&mut stream).unwrap();
            match msg {
                Message::MouseMove { x, y } => {
                    assert_eq!(x, expected_x);
                    assert_eq!(y, expected_x + 100);
                }
                _ => panic!("Expected MouseMove"),
            }
        }
        
        // Send KeepAlive back
        MessageCodec::write_to(&mut stream, &Message::KeepAlive).unwrap();
    });
    
    let mut client = TcpStream::connect(addr).unwrap();
    client.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    
    // Send multiple mouse moves
    for x in [100i16, 200, 300, 400, 500] {
        let msg = Message::MouseMove { x, y: x + 100 };
        MessageCodec::write_to(&mut client, &msg).unwrap();
    }
    
    // Receive KeepAlive
    let msg = MessageCodec::read_from(&mut client).unwrap();
    assert!(matches!(msg, Message::KeepAlive));
    
    server_handle.join().unwrap();
}

/// Test clipboard data transfer
#[test]
fn test_clipboard_transfer() {
    use macwinshare_core::protocol::ClipboardFormat;
    
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    
    let clipboard_text = "Hello from clipboard! 你好 🎉".as_bytes().to_vec();
    let clipboard_clone = clipboard_text.clone();
    
    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        
        // Receive clipboard grab
        let msg = MessageCodec::read_from(&mut stream).unwrap();
        match msg {
            Message::ClipboardGrab { clipboard_id, sequence_number } => {
                assert_eq!(clipboard_id, 0);
                assert_eq!(sequence_number, 1);
            }
            _ => panic!("Expected ClipboardGrab"),
        }
        
        // Receive clipboard data
        let msg = MessageCodec::read_from(&mut stream).unwrap();
        match msg {
            Message::ClipboardData { clipboard_id, sequence_number, format, data } => {
                assert_eq!(clipboard_id, 0);
                assert_eq!(sequence_number, 1);
                assert_eq!(format, ClipboardFormat::Text);
                assert_eq!(data, clipboard_clone);
            }
            _ => panic!("Expected ClipboardData"),
        }
    });
    
    let mut client = TcpStream::connect(addr).unwrap();
    
    // Send clipboard grab
    let grab = Message::ClipboardGrab {
        clipboard_id: 0,
        sequence_number: 1,
    };
    MessageCodec::write_to(&mut client, &grab).unwrap();
    
    // Send clipboard data
    let data = Message::ClipboardData {
        clipboard_id: 0,
        sequence_number: 1,
        format: ClipboardFormat::Text,
        data: clipboard_text,
    };
    MessageCodec::write_to(&mut client, &data).unwrap();
    
    server_handle.join().unwrap();
}

/// Test protocol message encoding size
#[test]
fn test_message_sizes() {
    // Hello message should be reasonably small
    let hello = Message::hello("TestMachine");
    let encoded = MessageCodec::encode(&hello);
    assert!(encoded.len() < 100, "Hello message too large: {} bytes", encoded.len());
    
    // MouseMove should be very compact
    let mouse = Message::MouseMove { x: 1920, y: 1080 };
    let encoded = MessageCodec::encode(&mouse);
    assert!(encoded.len() < 20, "MouseMove message too large: {} bytes", encoded.len());
    
    // KeepAlive should be minimal
    let keepalive = Message::KeepAlive;
    let encoded = MessageCodec::encode(&keepalive);
    assert!(encoded.len() < 10, "KeepAlive message too large: {} bytes", encoded.len());
}

/// Test encoding consistency
#[test]
fn test_encoding_consistency() {
    let msg = Message::MouseMove { x: 500, y: 600 };
    
    // Encode multiple times, should get same result
    let encoded1 = MessageCodec::encode(&msg);
    let encoded2 = MessageCodec::encode(&msg);
    let encoded3 = MessageCodec::encode(&msg);
    
    assert_eq!(encoded1, encoded2);
    assert_eq!(encoded2, encoded3);
}
