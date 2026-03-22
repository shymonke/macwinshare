//! Deskflow-compatible wire protocol
//! 
//! This implements a protocol compatible with Deskflow/Barrier/Synergy,
//! allowing interoperability with existing clients.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Protocol version
pub const PROTOCOL_MAJOR: u16 = 1;
pub const PROTOCOL_MINOR: u16 = 6;

/// Message types in the protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    // Connection messages
    Hello {
        major: u16,
        minor: u16,
        name: String,
    },
    HelloBack {
        major: u16,
        minor: u16,
        name: String,
    },
    
    // Input messages
    KeyDown {
        key_id: u16,
        modifier_mask: u16,
        key_button: u16,
    },
    KeyUp {
        key_id: u16,
        modifier_mask: u16,
        key_button: u16,
    },
    KeyRepeat {
        key_id: u16,
        modifier_mask: u16,
        key_button: u16,
        count: u16,
    },
    
    // Mouse messages
    MouseDown {
        button_id: i8,
    },
    MouseUp {
        button_id: i8,
    },
    MouseMove {
        x: i16,
        y: i16,
    },
    MouseRelativeMove {
        dx: i16,
        dy: i16,
    },
    MouseWheel {
        x_delta: i16,
        y_delta: i16,
    },
    
    // Screen switching
    Enter {
        x: i16,
        y: i16,
        sequence_number: u32,
        modifier_mask: u16,
    },
    Leave,
    
    // Clipboard
    ClipboardGrab {
        clipboard_id: u8,
        sequence_number: u32,
    },
    ClipboardData {
        clipboard_id: u8,
        sequence_number: u32,
        format: ClipboardFormat,
        data: Vec<u8>,
    },
    
    // Screen info
    ScreenInfo {
        x: i16,
        y: i16,
        width: i16,
        height: i16,
        cursor_x: i16,
        cursor_y: i16,
    },
    InfoAck,
    
    // Keep alive
    KeepAlive,
    
    // Options
    SetOptions {
        options: Vec<(String, u32)>,
    },
    ResetOptions,
    
    // Query
    QueryInfo,
    
    // Error/close
    Close,
    ErrorMsg {
        error: String,
    },
    
    // File transfer (extension)
    FileTransferStart {
        filename: String,
        size: u64,
        transfer_id: u32,
    },
    FileTransferData {
        transfer_id: u32,
        data: Vec<u8>,
    },
    FileTransferEnd {
        transfer_id: u32,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClipboardFormat {
    Text,
    Html,
    Bitmap,
    File,
}

/// Codec for encoding/decoding messages
pub struct MessageCodec;

impl MessageCodec {
    /// Encode a message to bytes (length-prefixed)
    pub fn encode(message: &Message) -> Vec<u8> {
        let payload = bincode::serialize(message).unwrap_or_default();
        let len = payload.len() as u32;
        
        let mut result = Vec::with_capacity(4 + payload.len());
        result.extend_from_slice(&len.to_be_bytes());
        result.extend_from_slice(&payload);
        result
    }

    /// Decode a message from bytes
    pub fn decode(data: &[u8]) -> crate::Result<Message> {
        if data.len() < 4 {
            return Err(crate::Error::InvalidMessage("Message too short".into()));
        }

        let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        
        if data.len() < 4 + len {
            return Err(crate::Error::InvalidMessage("Incomplete message".into()));
        }

        let payload = &data[4..4 + len];
        bincode::deserialize(payload)
            .map_err(|e| crate::Error::InvalidMessage(e.to_string()))
    }

    /// Read a message from a stream
    pub fn read_from<R: Read>(reader: &mut R) -> crate::Result<Message> {
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut payload = vec![0u8; len];
        reader.read_exact(&mut payload)?;

        bincode::deserialize(&payload)
            .map_err(|e| crate::Error::InvalidMessage(e.to_string()))
    }

    /// Write a message to a stream
    pub fn write_to<W: Write>(writer: &mut W, message: &Message) -> crate::Result<()> {
        let data = Self::encode(message);
        writer.write_all(&data)?;
        Ok(())
    }
}

/// Key modifier flags (compatible with Synergy/Deskflow)
pub mod modifiers {
    pub const SHIFT: u16 = 0x0001;
    pub const CTRL: u16 = 0x0002;
    pub const ALT: u16 = 0x0004;
    pub const META: u16 = 0x0008;  // Windows key / Command key
    pub const SUPER: u16 = 0x0010;
    pub const CAPS_LOCK: u16 = 0x1000;
    pub const NUM_LOCK: u16 = 0x2000;
    pub const SCROLL_LOCK: u16 = 0x4000;
}

/// Mouse button IDs
pub mod buttons {
    pub const LEFT: i8 = 1;
    pub const MIDDLE: i8 = 2;
    pub const RIGHT: i8 = 3;
    pub const BACK: i8 = 4;
    pub const FORWARD: i8 = 5;
}

/// Common key codes (virtual key codes)
pub mod keys {
    // Modifier keys
    pub const SHIFT_L: u16 = 0xFFE1;
    pub const SHIFT_R: u16 = 0xFFE2;
    pub const CTRL_L: u16 = 0xFFE3;
    pub const CTRL_R: u16 = 0xFFE4;
    pub const ALT_L: u16 = 0xFFE9;
    pub const ALT_R: u16 = 0xFFEA;
    pub const META_L: u16 = 0xFFE7;
    pub const META_R: u16 = 0xFFE8;
    pub const SUPER_L: u16 = 0xFFEB;
    pub const SUPER_R: u16 = 0xFFEC;

    // Function keys
    pub const F1: u16 = 0xFFBE;
    pub const F2: u16 = 0xFFBF;
    pub const F3: u16 = 0xFFC0;
    pub const F4: u16 = 0xFFC1;
    pub const F5: u16 = 0xFFC2;
    pub const F6: u16 = 0xFFC3;
    pub const F7: u16 = 0xFFC4;
    pub const F8: u16 = 0xFFC5;
    pub const F9: u16 = 0xFFC6;
    pub const F10: u16 = 0xFFC7;
    pub const F11: u16 = 0xFFC8;
    pub const F12: u16 = 0xFFC9;

    // Navigation
    pub const ESCAPE: u16 = 0xFF1B;
    pub const TAB: u16 = 0xFF09;
    pub const RETURN: u16 = 0xFF0D;
    pub const BACKSPACE: u16 = 0xFF08;
    pub const DELETE: u16 = 0xFFFF;
    pub const INSERT: u16 = 0xFF63;
    pub const HOME: u16 = 0xFF50;
    pub const END: u16 = 0xFF57;
    pub const PAGE_UP: u16 = 0xFF55;
    pub const PAGE_DOWN: u16 = 0xFF56;
    pub const UP: u16 = 0xFF52;
    pub const DOWN: u16 = 0xFF54;
    pub const LEFT: u16 = 0xFF51;
    pub const RIGHT: u16 = 0xFF53;

    // Locks
    pub const CAPS_LOCK: u16 = 0xFFE5;
    pub const NUM_LOCK: u16 = 0xFF7F;
    pub const SCROLL_LOCK: u16 = 0xFF14;

    // Space and others
    pub const SPACE: u16 = 0x0020;
}

impl Message {
    /// Create a hello message for connection initiation
    pub fn hello(name: &str) -> Self {
        Message::Hello {
            major: PROTOCOL_MAJOR,
            minor: PROTOCOL_MINOR,
            name: name.to_string(),
        }
    }

    /// Create a hello back message for connection response
    pub fn hello_back(name: &str) -> Self {
        Message::HelloBack {
            major: PROTOCOL_MAJOR,
            minor: PROTOCOL_MINOR,
            name: name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to test roundtrip encoding/decoding
    fn test_roundtrip(msg: Message) -> Message {
        let encoded = MessageCodec::encode(&msg);
        MessageCodec::decode(&encoded).expect("Failed to decode message")
    }

    #[test]
    fn test_encode_decode_hello() {
        let msg = Message::hello("TestMachine");
        let encoded = MessageCodec::encode(&msg);
        let decoded = MessageCodec::decode(&encoded).unwrap();
        
        match decoded {
            Message::Hello { major, minor, name } => {
                assert_eq!(major, PROTOCOL_MAJOR);
                assert_eq!(minor, PROTOCOL_MINOR);
                assert_eq!(name, "TestMachine");
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_encode_decode_mouse_move() {
        let msg = Message::MouseMove { x: 100, y: 200 };
        let encoded = MessageCodec::encode(&msg);
        let decoded = MessageCodec::decode(&encoded).unwrap();
        
        match decoded {
            Message::MouseMove { x, y } => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_hello_back() {
        let decoded = test_roundtrip(Message::hello_back("ServerName"));
        match decoded {
            Message::HelloBack { major, minor, name } => {
                assert_eq!(major, PROTOCOL_MAJOR);
                assert_eq!(minor, PROTOCOL_MINOR);
                assert_eq!(name, "ServerName");
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_key_down() {
        let decoded = test_roundtrip(Message::KeyDown {
            key_id: keys::SPACE,
            modifier_mask: modifiers::CTRL | modifiers::SHIFT,
            key_button: 49,
        });
        match decoded {
            Message::KeyDown { key_id, modifier_mask, key_button } => {
                assert_eq!(key_id, keys::SPACE);
                assert_eq!(modifier_mask, modifiers::CTRL | modifiers::SHIFT);
                assert_eq!(key_button, 49);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_key_up() {
        let decoded = test_roundtrip(Message::KeyUp {
            key_id: keys::RETURN,
            modifier_mask: 0,
            key_button: 36,
        });
        match decoded {
            Message::KeyUp { key_id, modifier_mask, key_button } => {
                assert_eq!(key_id, keys::RETURN);
                assert_eq!(modifier_mask, 0);
                assert_eq!(key_button, 36);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_key_repeat() {
        let decoded = test_roundtrip(Message::KeyRepeat {
            key_id: keys::F1,
            modifier_mask: modifiers::ALT,
            key_button: 122,
            count: 5,
        });
        match decoded {
            Message::KeyRepeat { key_id, modifier_mask, key_button, count } => {
                assert_eq!(key_id, keys::F1);
                assert_eq!(modifier_mask, modifiers::ALT);
                assert_eq!(key_button, 122);
                assert_eq!(count, 5);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_mouse_buttons() {
        // Mouse down
        let decoded = test_roundtrip(Message::MouseDown { button_id: buttons::LEFT });
        match decoded {
            Message::MouseDown { button_id } => assert_eq!(button_id, buttons::LEFT),
            _ => panic!("Unexpected message type"),
        }

        // Mouse up
        let decoded = test_roundtrip(Message::MouseUp { button_id: buttons::RIGHT });
        match decoded {
            Message::MouseUp { button_id } => assert_eq!(button_id, buttons::RIGHT),
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_mouse_relative_move() {
        let decoded = test_roundtrip(Message::MouseRelativeMove { dx: -50, dy: 30 });
        match decoded {
            Message::MouseRelativeMove { dx, dy } => {
                assert_eq!(dx, -50);
                assert_eq!(dy, 30);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_mouse_wheel() {
        let decoded = test_roundtrip(Message::MouseWheel { x_delta: 0, y_delta: -120 });
        match decoded {
            Message::MouseWheel { x_delta, y_delta } => {
                assert_eq!(x_delta, 0);
                assert_eq!(y_delta, -120);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_enter_leave() {
        let decoded = test_roundtrip(Message::Enter {
            x: 1920,
            y: 540,
            sequence_number: 12345,
            modifier_mask: modifiers::CAPS_LOCK,
        });
        match decoded {
            Message::Enter { x, y, sequence_number, modifier_mask } => {
                assert_eq!(x, 1920);
                assert_eq!(y, 540);
                assert_eq!(sequence_number, 12345);
                assert_eq!(modifier_mask, modifiers::CAPS_LOCK);
            }
            _ => panic!("Unexpected message type"),
        }

        let decoded = test_roundtrip(Message::Leave);
        assert!(matches!(decoded, Message::Leave));
    }

    #[test]
    fn test_clipboard_grab() {
        let decoded = test_roundtrip(Message::ClipboardGrab {
            clipboard_id: 0,
            sequence_number: 999,
        });
        match decoded {
            Message::ClipboardGrab { clipboard_id, sequence_number } => {
                assert_eq!(clipboard_id, 0);
                assert_eq!(sequence_number, 999);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_clipboard_data() {
        let data = b"Hello, clipboard!".to_vec();
        let decoded = test_roundtrip(Message::ClipboardData {
            clipboard_id: 0,
            sequence_number: 100,
            format: ClipboardFormat::Text,
            data: data.clone(),
        });
        match decoded {
            Message::ClipboardData { clipboard_id, sequence_number, format, data: decoded_data } => {
                assert_eq!(clipboard_id, 0);
                assert_eq!(sequence_number, 100);
                assert_eq!(format, ClipboardFormat::Text);
                assert_eq!(decoded_data, data);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_screen_info() {
        let decoded = test_roundtrip(Message::ScreenInfo {
            x: 0,
            y: 0,
            width: 2560,
            height: 1440,
            cursor_x: 1280,
            cursor_y: 720,
        });
        match decoded {
            Message::ScreenInfo { x, y, width, height, cursor_x, cursor_y } => {
                assert_eq!(x, 0);
                assert_eq!(y, 0);
                assert_eq!(width, 2560);
                assert_eq!(height, 1440);
                assert_eq!(cursor_x, 1280);
                assert_eq!(cursor_y, 720);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_info_ack_and_keepalive() {
        assert!(matches!(test_roundtrip(Message::InfoAck), Message::InfoAck));
        assert!(matches!(test_roundtrip(Message::KeepAlive), Message::KeepAlive));
        assert!(matches!(test_roundtrip(Message::QueryInfo), Message::QueryInfo));
        assert!(matches!(test_roundtrip(Message::ResetOptions), Message::ResetOptions));
        assert!(matches!(test_roundtrip(Message::Close), Message::Close));
    }

    #[test]
    fn test_set_options() {
        let options = vec![
            ("heartbeat".to_string(), 5000u32),
            ("screensaver".to_string(), 1u32),
        ];
        let decoded = test_roundtrip(Message::SetOptions { options: options.clone() });
        match decoded {
            Message::SetOptions { options: decoded_options } => {
                assert_eq!(decoded_options, options);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_error_msg() {
        let decoded = test_roundtrip(Message::ErrorMsg {
            error: "Connection refused".to_string(),
        });
        match decoded {
            Message::ErrorMsg { error } => {
                assert_eq!(error, "Connection refused");
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_file_transfer() {
        // Start
        let decoded = test_roundtrip(Message::FileTransferStart {
            filename: "document.pdf".to_string(),
            size: 1024 * 1024,
            transfer_id: 42,
        });
        match decoded {
            Message::FileTransferStart { filename, size, transfer_id } => {
                assert_eq!(filename, "document.pdf");
                assert_eq!(size, 1024 * 1024);
                assert_eq!(transfer_id, 42);
            }
            _ => panic!("Unexpected message type"),
        }

        // Data
        let data = vec![0u8; 4096];
        let decoded = test_roundtrip(Message::FileTransferData {
            transfer_id: 42,
            data: data.clone(),
        });
        match decoded {
            Message::FileTransferData { transfer_id, data: decoded_data } => {
                assert_eq!(transfer_id, 42);
                assert_eq!(decoded_data.len(), 4096);
            }
            _ => panic!("Unexpected message type"),
        }

        // End
        let decoded = test_roundtrip(Message::FileTransferEnd { transfer_id: 42 });
        match decoded {
            Message::FileTransferEnd { transfer_id } => {
                assert_eq!(transfer_id, 42);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_edge_values() {
        // Test maximum/minimum i16 values for coordinates
        let decoded = test_roundtrip(Message::MouseMove { x: i16::MAX, y: i16::MIN });
        match decoded {
            Message::MouseMove { x, y } => {
                assert_eq!(x, i16::MAX);
                assert_eq!(y, i16::MIN);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_unicode_in_strings() {
        let decoded = test_roundtrip(Message::hello("测试机器-émoji-🖥️"));
        match decoded {
            Message::Hello { name, .. } => {
                assert_eq!(name, "测试机器-émoji-🖥️");
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_empty_string() {
        let decoded = test_roundtrip(Message::hello(""));
        match decoded {
            Message::Hello { name, .. } => {
                assert_eq!(name, "");
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_decode_too_short() {
        let result = MessageCodec::decode(&[0, 0, 0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_incomplete_payload() {
        // Header says 100 bytes, but only 10 provided
        let mut data = vec![0, 0, 0, 100];
        data.extend_from_slice(&[0u8; 10]);
        let result = MessageCodec::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_write_stream() {
        let msg = Message::MouseMove { x: 500, y: 600 };
        let mut buffer = Vec::new();
        
        MessageCodec::write_to(&mut buffer, &msg).unwrap();
        
        let mut cursor = std::io::Cursor::new(&buffer);
        let decoded = MessageCodec::read_from(&mut cursor).unwrap();
        
        match decoded {
            Message::MouseMove { x, y } => {
                assert_eq!(x, 500);
                assert_eq!(y, 600);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_clipboard_formats() {
        for format in [ClipboardFormat::Text, ClipboardFormat::Html, ClipboardFormat::Bitmap, ClipboardFormat::File] {
            let decoded = test_roundtrip(Message::ClipboardData {
                clipboard_id: 0,
                sequence_number: 1,
                format,
                data: vec![1, 2, 3],
            });
            match decoded {
                Message::ClipboardData { format: decoded_format, .. } => {
                    assert_eq!(decoded_format, format);
                }
                _ => panic!("Unexpected message type"),
            }
        }
    }
}
