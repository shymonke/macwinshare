//! MacWinShare Server
//! 
//! The server is the machine with the primary keyboard and mouse.
//! It captures input and forwards it to connected clients.

use crate::clipboard::ClipboardManager;
use crate::config::Config;
use crate::protocol::{Message, MessageCodec};
use crate::screen::ScreenEdgeDetector;
use crate::{Error, Result};
use rustls::ServerConfig;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_rustls::TlsAcceptor;
use tracing::{debug, error, info, warn};

/// Connected client information
struct ConnectedClient {
    name: String,
    addr: SocketAddr,
    tx: mpsc::Sender<Message>,
}

/// The MacWinShare server
pub struct Server {
    config: Arc<RwLock<Config>>,
    tls_acceptor: TlsAcceptor,
    clients: Arc<RwLock<HashMap<String, ConnectedClient>>>,
    active_client: Arc<RwLock<Option<String>>>,
}

impl Server {
    /// Create a new server instance
    pub async fn new(
        config: Arc<RwLock<Config>>,
        tls_config: Arc<ServerConfig>,
    ) -> Result<Self> {
        let tls_acceptor = TlsAcceptor::from(tls_config);

        Ok(Self {
            config,
            tls_acceptor,
            clients: Arc::new(RwLock::new(HashMap::new())),
            active_client: Arc::new(RwLock::new(None)),
        })
    }

    /// Run the server
    pub async fn run(self) -> Result<()> {
        let config = self.config.read().await;
        let addr = format!("0.0.0.0:{}", config.port);
        drop(config);

        let listener = TcpListener::bind(&addr).await?;
        info!("Server listening on {}", addr);

        // Start clipboard monitoring
        let (clipboard_manager, mut clipboard_rx) = ClipboardManager::new();
        let clients_for_clipboard = self.clients.clone();
        
        tokio::spawn(async move {
            clipboard_manager.start_monitoring().await.ok();
            
            while let Some(content) = clipboard_rx.recv().await {
                let clients = clients_for_clipboard.read().await;
                for client in clients.values() {
                    // Convert clipboard content to protocol message
                    if let crate::clipboard::ClipboardContent::Text(text) = &content {
                        let msg = Message::ClipboardData {
                            clipboard_id: 0,
                            sequence_number: 0,
                            format: crate::protocol::ClipboardFormat::Text,
                            data: text.as_bytes().to_vec(),
                        };
                        let _ = client.tx.send(msg).await;
                    }
                }
            }
        });

        // Start input capture (platform-specific)
        self.start_input_capture().await?;

        // Accept connections
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("New connection from {}", addr);
                    let server = Server {
                        config: self.config.clone(),
                        tls_acceptor: self.tls_acceptor.clone(),
                        clients: self.clients.clone(),
                        active_client: self.active_client.clone(),
                    };
                    
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream, addr).await {
                            error!("Connection error from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }

    async fn handle_connection(&self, stream: TcpStream, addr: SocketAddr) -> Result<()> {
        // TLS handshake
        let tls_stream = self.tls_acceptor.accept(stream).await?;
        let (mut reader, mut writer) = tokio::io::split(tls_stream);

        // Read hello message
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        let mut payload = vec![0u8; len];
        reader.read_exact(&mut payload).await?;
        
        let hello: Message = bincode::deserialize(&payload)
            .map_err(|e| Error::Protocol(e.to_string()))?;

        let client_name = match hello {
            Message::Hello { name, major, minor, .. } => {
                info!("Client {} connected (protocol {}.{})", name, major, minor);
                name
            }
            _ => {
                return Err(Error::Protocol("Expected Hello message".into()));
            }
        };

        // Send hello back
        let config = self.config.read().await;
        let hello_back = Message::hello_back(&config.machine_name);
        let response = MessageCodec::encode(&hello_back);
        writer.write_all(&response).await?;
        drop(config);

        // Create message channel for this client
        let (tx, mut rx) = mpsc::channel::<Message>(32);

        // Register client
        {
            let mut clients = self.clients.write().await;
            clients.insert(client_name.clone(), ConnectedClient {
                name: client_name.clone(),
                addr,
                tx,
            });
        }

        // Spawn writer task
        let client_name_writer = client_name.clone();
        let writer_handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let data = MessageCodec::encode(&msg);
                if let Err(e) = writer.write_all(&data).await {
                    warn!("Error writing to {}: {}", client_name_writer, e);
                    break;
                }
            }
        });

        // Read messages from client
        loop {
            let mut len_buf = [0u8; 4];
            match reader.read_exact(&mut len_buf).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    info!("Client {} disconnected", client_name);
                    break;
                }
                Err(e) => {
                    error!("Read error from {}: {}", client_name, e);
                    break;
                }
            }

            let len = u32::from_be_bytes(len_buf) as usize;
            let mut payload = vec![0u8; len];
            reader.read_exact(&mut payload).await?;

            let message: Message = bincode::deserialize(&payload)
                .map_err(|e| Error::Protocol(e.to_string()))?;

            self.handle_client_message(&client_name, message).await?;
        }

        // Cleanup
        {
            let mut clients = self.clients.write().await;
            clients.remove(&client_name);
        }

        writer_handle.abort();
        Ok(())
    }

    async fn handle_client_message(&self, client_name: &str, message: Message) -> Result<()> {
        match message {
            Message::ScreenInfo { width, height, .. } => {
                debug!("Client {} screen: {}x{}", client_name, width, height);
            }
            Message::ClipboardData { data, format, .. } => {
                debug!("Received clipboard data from {}", client_name);
                // Apply clipboard content locally
                if format == crate::protocol::ClipboardFormat::Text {
                    if let Ok(text) = String::from_utf8(data) {
                        let manager = ClipboardManager::new().0;
                        manager.set_content(&crate::clipboard::ClipboardContent::Text(text)).ok();
                    }
                }
            }
            Message::KeepAlive => {
                debug!("Keep-alive from {}", client_name);
            }
            _ => {
                debug!("Received message from {}: {:?}", client_name, message);
            }
        }
        Ok(())
    }

    async fn start_input_capture(&self) -> Result<()> {
        let clients = self.clients.clone();
        let active_client = self.active_client.clone();
        let config = self.config.clone();

        // Platform-specific input capture would go here
        // For now, we'll set up the screen edge detector
        
        let cfg = config.read().await;
        let edge_detector = ScreenEdgeDetector::new(cfg.screen.clone());
        drop(cfg);

        // In a full implementation, we would:
        // 1. Hook into system-level mouse/keyboard events
        // 2. Check for screen edges
        // 3. Forward events to the active client

        info!("Input capture started (stub - platform implementation needed)");
        Ok(())
    }

    /// Send an input event to the active client
    pub async fn send_to_active(&self, message: Message) -> Result<()> {
        let active = self.active_client.read().await;
        if let Some(ref name) = *active {
            let clients = self.clients.read().await;
            if let Some(client) = clients.get(name) {
                client.tx.send(message).await
                    .map_err(|_| Error::Connection("Failed to send to client".into()))?;
            }
        }
        Ok(())
    }

    /// Switch to a specific client
    pub async fn switch_to(&self, client_name: &str) -> Result<()> {
        let clients = self.clients.read().await;
        if clients.contains_key(client_name) {
            drop(clients);
            
            // Leave current client
            let mut active = self.active_client.write().await;
            if let Some(ref current) = *active {
                let clients = self.clients.read().await;
                if let Some(client) = clients.get(current) {
                    let _ = client.tx.send(Message::Leave).await;
                }
            }

            // Enter new client
            *active = Some(client_name.to_string());
            drop(active);

            let clients = self.clients.read().await;
            if let Some(client) = clients.get(client_name) {
                let _ = client.tx.send(Message::Enter {
                    x: 100,
                    y: 100,
                    sequence_number: 0,
                    modifier_mask: 0,
                }).await;
            }

            info!("Switched to client: {}", client_name);
            Ok(())
        } else {
            Err(Error::Connection(format!("Client {} not found", client_name)))
        }
    }
}
