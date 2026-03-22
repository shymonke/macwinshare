//! MacWinShare Client
//! 
//! The client receives input events from the server and injects them locally.

use crate::clipboard::ClipboardManager;
use crate::config::Config;
use crate::protocol::{Message, MessageCodec};
use crate::{Error, Result};
use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_rustls::TlsConnector;
use tracing::{debug, error, info, warn};

/// The MacWinShare client
pub struct Client {
    config: Arc<RwLock<Config>>,
    tls_connector: TlsConnector,
    is_active: Arc<RwLock<bool>>,
}

impl Client {
    /// Create a new client instance
    pub async fn new(
        config: Arc<RwLock<Config>>,
        tls_config: Arc<ClientConfig>,
    ) -> Result<Self> {
        let tls_connector = TlsConnector::from(tls_config);

        Ok(Self {
            config,
            tls_connector,
            is_active: Arc::new(RwLock::new(false)),
        })
    }

    /// Connect to a server
    pub async fn connect(&self, server_addr: &str) -> Result<()> {
        info!("Connecting to server at {}", server_addr);

        // Parse address
        let addr: std::net::SocketAddr = server_addr.parse()
            .map_err(|e| Error::Connection(format!("Invalid address: {}", e)))?;

        // Connect TCP
        let stream = TcpStream::connect(addr).await?;
        info!("TCP connected to {}", addr);

        // TLS handshake
        let server_name = ServerName::try_from("localhost")
            .map_err(|_| Error::Connection("Invalid server name".into()))?;
        
        let tls_stream = self.tls_connector.connect(server_name, stream).await?;
        info!("TLS handshake completed");

        let (mut reader, mut writer) = tokio::io::split(tls_stream);

        // Send hello
        let config = self.config.read().await;
        let hello = Message::hello(&config.machine_name);
        let hello_data = MessageCodec::encode(&hello);
        writer.write_all(&hello_data).await?;
        drop(config);

        // Read hello back
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        let mut payload = vec![0u8; len];
        reader.read_exact(&mut payload).await?;

        let hello_back: Message = bincode::deserialize(&payload)
            .map_err(|e| Error::Protocol(e.to_string()))?;

        match hello_back {
            Message::HelloBack { name, major, minor } => {
                info!("Connected to server {} (protocol {}.{})", name, major, minor);
            }
            _ => {
                return Err(Error::Protocol("Expected HelloBack message".into()));
            }
        }

        // Send screen info
        let config = self.config.read().await;
        let screen_info = Message::ScreenInfo {
            x: config.screen.position.x as i16,
            y: config.screen.position.y as i16,
            width: config.screen.width as i16,
            height: config.screen.height as i16,
            cursor_x: (config.screen.width / 2) as i16,
            cursor_y: (config.screen.height / 2) as i16,
        };
        let screen_data = MessageCodec::encode(&screen_info);
        writer.write_all(&screen_data).await?;
        drop(config);

        // Start clipboard monitoring
        let (clipboard_manager, mut clipboard_rx) = ClipboardManager::new();
        let writer = Arc::new(tokio::sync::Mutex::new(writer));
        let writer_for_clipboard = writer.clone();

        tokio::spawn(async move {
            clipboard_manager.start_monitoring().await.ok();
            
            while let Some(content) = clipboard_rx.recv().await {
                if let crate::clipboard::ClipboardContent::Text(text) = &content {
                    let msg = Message::ClipboardData {
                        clipboard_id: 0,
                        sequence_number: 0,
                        format: crate::protocol::ClipboardFormat::Text,
                        data: text.as_bytes().to_vec(),
                    };
                    let data = MessageCodec::encode(&msg);
                    let mut w = writer_for_clipboard.lock().await;
                    let _ = w.write_all(&data).await;
                }
            }
        });

        // Start keep-alive task
        let writer_for_keepalive = writer.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
            loop {
                interval.tick().await;
                let msg = Message::KeepAlive;
                let data = MessageCodec::encode(&msg);
                let mut w = writer_for_keepalive.lock().await;
                if w.write_all(&data).await.is_err() {
                    break;
                }
            }
        });

        // Main message loop
        loop {
            let mut len_buf = [0u8; 4];
            match reader.read_exact(&mut len_buf).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    info!("Server disconnected");
                    break;
                }
                Err(e) => {
                    error!("Read error: {}", e);
                    break;
                }
            }

            let len = u32::from_be_bytes(len_buf) as usize;
            let mut payload = vec![0u8; len];
            reader.read_exact(&mut payload).await?;

            let message: Message = bincode::deserialize(&payload)
                .map_err(|e| Error::Protocol(e.to_string()))?;

            self.handle_server_message(message).await?;
        }

        Ok(())
    }

    async fn handle_server_message(&self, message: Message) -> Result<()> {
        match message {
            Message::Enter { x, y, .. } => {
                info!("Cursor entering at ({}, {})", x, y);
                let mut active = self.is_active.write().await;
                *active = true;
                
                // Platform-specific: show cursor, position it
                // self.platform.set_cursor_position(CursorPosition::new(x as i32, y as i32))?;
            }
            
            Message::Leave => {
                info!("Cursor leaving");
                let mut active = self.is_active.write().await;
                *active = false;
                
                // Platform-specific: hide cursor or center it
            }
            
            Message::MouseMove { x, y } => {
                let active = self.is_active.read().await;
                if *active {
                    debug!("Mouse move to ({}, {})", x, y);
                    // Platform-specific: move cursor
                    // self.platform.set_cursor_position(CursorPosition::new(x as i32, y as i32))?;
                }
            }
            
            Message::MouseDown { button_id } => {
                let active = self.is_active.read().await;
                if *active {
                    debug!("Mouse down: button {}", button_id);
                    // Platform-specific: inject mouse down
                }
            }
            
            Message::MouseUp { button_id } => {
                let active = self.is_active.read().await;
                if *active {
                    debug!("Mouse up: button {}", button_id);
                    // Platform-specific: inject mouse up
                }
            }
            
            Message::MouseWheel { x_delta, y_delta } => {
                let active = self.is_active.read().await;
                if *active {
                    debug!("Mouse wheel: ({}, {})", x_delta, y_delta);
                    // Platform-specific: inject scroll
                }
            }
            
            Message::KeyDown { key_id, modifier_mask, key_button } => {
                let active = self.is_active.read().await;
                if *active {
                    debug!("Key down: {} (modifiers: {:04x})", key_id, modifier_mask);
                    // Platform-specific: inject key down
                }
            }
            
            Message::KeyUp { key_id, modifier_mask, key_button } => {
                let active = self.is_active.read().await;
                if *active {
                    debug!("Key up: {} (modifiers: {:04x})", key_id, modifier_mask);
                    // Platform-specific: inject key up
                }
            }
            
            Message::ClipboardData { data, format, .. } => {
                debug!("Received clipboard data ({} bytes)", data.len());
                if format == crate::protocol::ClipboardFormat::Text {
                    if let Ok(text) = String::from_utf8(data) {
                        let manager = ClipboardManager::new().0;
                        manager.set_content(&crate::clipboard::ClipboardContent::Text(text)).ok();
                    }
                }
            }
            
            Message::KeepAlive => {
                debug!("Keep-alive from server");
            }
            
            Message::SetOptions { options } => {
                debug!("Server options: {:?}", options);
            }
            
            _ => {
                debug!("Unhandled message: {:?}", message);
            }
        }
        
        Ok(())
    }
}
