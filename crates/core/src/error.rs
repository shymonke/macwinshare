//! Error types for MacWinShare

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TLS error: {0}")]
    Tls(#[from] rustls::Error),

    #[error("Discovery error: {0}")]
    Discovery(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Clipboard error: {0}")]
    Clipboard(String),

    #[error("Certificate error: {0}")]
    Certificate(String),

    #[error("Timeout")]
    Timeout,

    #[error("Not connected")]
    NotConnected,

    #[error("Already connected")]
    AlreadyConnected,

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Peer rejected: {0}")]
    PeerRejected(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
