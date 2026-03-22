//! Clipboard sharing between connected machines

use crate::{Error, Result};
use arboard::Clipboard;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, warn};

/// Clipboard content that can be shared
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipboardContent {
    Text(String),
    Image {
        width: u32,
        height: u32,
        rgba_data: Vec<u8>,
    },
    Files(Vec<String>),
}

/// Manages clipboard synchronization
pub struct ClipboardManager {
    last_content_hash: Arc<RwLock<Option<u64>>>,
    tx: mpsc::Sender<ClipboardContent>,
}

impl ClipboardManager {
    /// Create a new clipboard manager
    pub fn new() -> (Self, mpsc::Receiver<ClipboardContent>) {
        let (tx, rx) = mpsc::channel(16);
        
        let manager = Self {
            last_content_hash: Arc::new(RwLock::new(None)),
            tx,
        };
        
        (manager, rx)
    }

    /// Start monitoring the clipboard for changes
    pub async fn start_monitoring(&self) -> Result<()> {
        let hash = self.last_content_hash.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let mut clipboard = match Clipboard::new() {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to access clipboard: {}", e);
                    return;
                }
            };

            loop {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;

                // Check for text changes
                if let Ok(text) = clipboard.get_text() {
                    let new_hash = Self::hash_content(&text);
                    let mut current_hash = hash.write().await;
                    
                    if *current_hash != Some(new_hash) {
                        *current_hash = Some(new_hash);
                        drop(current_hash);
                        
                        debug!("Clipboard text changed, broadcasting");
                        let _ = tx.send(ClipboardContent::Text(text)).await;
                    }
                }

                // Check for image changes
                #[cfg(feature = "image")]
                if let Ok(image) = clipboard.get_image() {
                    let new_hash = Self::hash_bytes(&image.bytes);
                    let mut current_hash = hash.write().await;
                    
                    if *current_hash != Some(new_hash) {
                        *current_hash = Some(new_hash);
                        drop(current_hash);
                        
                        debug!("Clipboard image changed, broadcasting");
                        let _ = tx.send(ClipboardContent::Image {
                            width: image.width as u32,
                            height: image.height as u32,
                            rgba_data: image.bytes.to_vec(),
                        }).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// Set the clipboard content (received from remote)
    pub fn set_content(&self, content: &ClipboardContent) -> Result<()> {
        let mut clipboard = Clipboard::new()
            .map_err(|e| Error::Clipboard(e.to_string()))?;

        match content {
            ClipboardContent::Text(text) => {
                clipboard.set_text(text)
                    .map_err(|e| Error::Clipboard(e.to_string()))?;
                debug!("Set clipboard text: {} chars", text.len());
            }
            ClipboardContent::Image { width, height, rgba_data } => {
                let image_data = arboard::ImageData {
                    width: *width as usize,
                    height: *height as usize,
                    bytes: std::borrow::Cow::Borrowed(rgba_data),
                };
                clipboard.set_image(image_data)
                    .map_err(|e| Error::Clipboard(e.to_string()))?;
                debug!("Set clipboard image: {}x{}", width, height);
            }
            ClipboardContent::Files(files) => {
                // File paths are shared as text for now
                let text = files.join("\n");
                clipboard.set_text(&text)
                    .map_err(|e| Error::Clipboard(e.to_string()))?;
                debug!("Set clipboard files: {} items", files.len());
            }
        }

        Ok(())
    }

    /// Get the current clipboard content
    pub fn get_content(&self) -> Result<Option<ClipboardContent>> {
        let mut clipboard = Clipboard::new()
            .map_err(|e| Error::Clipboard(e.to_string()))?;

        // Try text first
        if let Ok(text) = clipboard.get_text() {
            if !text.is_empty() {
                return Ok(Some(ClipboardContent::Text(text)));
            }
        }

        // Try image
        #[cfg(feature = "image")]
        if let Ok(image) = clipboard.get_image() {
            return Ok(Some(ClipboardContent::Image {
                width: image.width as u32,
                height: image.height as u32,
                rgba_data: image.bytes.to_vec(),
            }));
        }

        Ok(None)
    }

    fn hash_content(text: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    fn hash_bytes(bytes: &[u8]) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        hasher.finish()
    }
}
