//! MacWinShare Core Library
//! 
//! Cross-platform keyboard and mouse sharing between Mac and Windows computers.
//! 
//! ## Architecture
//! 
//! - `discovery`: mDNS/UDP auto-discovery of peers on the network
//! - `protocol`: Deskflow-compatible wire protocol for input events
//! - `encryption`: TLS 1.3 encryption using rustls
//! - `clipboard`: Cross-platform clipboard synchronization
//! - `screen`: Screen edge detection and cursor management
//! - `config`: Application configuration management

pub mod config;
pub mod discovery;
pub mod encryption;
pub mod protocol;
pub mod clipboard;
pub mod screen;
pub mod platform;
pub mod server;
pub mod client;
pub mod error;

pub use config::Config;
pub use discovery::Discovery;
pub use encryption::TlsManager;
pub use protocol::{Message, MessageCodec};
pub use error::{Error, Result};

use std::sync::Arc;
use tokio::sync::RwLock;

/// The main MacWinShare application state
pub struct MacWinShare {
    pub config: Arc<RwLock<Config>>,
    pub discovery: Discovery,
    pub tls: TlsManager,
    mode: AppMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Server,
    Client,
    Auto,
}

impl MacWinShare {
    /// Create a new MacWinShare instance
    pub async fn new(config: Config) -> Result<Self> {
        let config = Arc::new(RwLock::new(config));
        let discovery = Discovery::new(config.clone()).await?;
        let tls = TlsManager::new(&*config.read().await)?;

        Ok(Self {
            config,
            discovery,
            tls,
            mode: AppMode::Auto,
        })
    }

    /// Start the application in server mode
    pub async fn start_server(&mut self) -> Result<()> {
        self.mode = AppMode::Server;
        tracing::info!("Starting MacWinShare server...");
        
        // Start discovery broadcasting
        self.discovery.start_advertising().await?;
        
        // Start the server
        let server = server::Server::new(
            self.config.clone(),
            self.tls.server_config()?,
        ).await?;
        
        server.run().await
    }

    /// Start the application in client mode
    pub async fn start_client(&mut self, server_addr: Option<String>) -> Result<()> {
        self.mode = AppMode::Client;
        tracing::info!("Starting MacWinShare client...");
        
        let addr = match server_addr {
            Some(addr) => addr,
            None => {
                // Discover server automatically
                self.discovery.find_server().await?
            }
        };

        let client = client::Client::new(
            self.config.clone(),
            self.tls.client_config()?,
        ).await?;
        
        client.connect(&addr).await
    }

    /// Get the current mode
    pub fn mode(&self) -> AppMode {
        self.mode
    }
}
