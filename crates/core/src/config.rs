//! Configuration management for MacWinShare

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Unique identifier for this machine
    pub machine_id: String,
    
    /// Human-readable name for this machine
    pub machine_name: String,
    
    /// Port to listen on (default: 24800 for Deskflow compatibility)
    pub port: u16,
    
    /// Screen configuration
    pub screen: ScreenConfig,
    
    /// Security settings
    pub security: SecurityConfig,
    
    /// Network settings
    pub network: NetworkConfig,
    
    /// Clipboard settings
    pub clipboard: ClipboardConfig,
    
    /// Path to store certificates and config
    pub data_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenConfig {
    /// This screen's position in the virtual desktop
    pub position: ScreenPosition,
    
    /// Screen width in pixels
    pub width: i32,
    
    /// Screen height in pixels  
    pub height: i32,
    
    /// Dead zone at edges (pixels) before switching
    pub edge_threshold: i32,
    
    /// Connected screens (name -> direction)
    pub neighbors: Vec<ScreenNeighbor>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScreenPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenNeighbor {
    pub name: String,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable TLS encryption (default: true)
    pub tls_enabled: bool,
    
    /// Require client certificate verification
    pub verify_clients: bool,
    
    /// Trusted certificate fingerprints
    pub trusted_fingerprints: Vec<String>,
    
    /// Optional password for additional security
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable mDNS discovery
    pub mdns_enabled: bool,
    
    /// Enable UDP broadcast discovery fallback
    pub udp_discovery_enabled: bool,
    
    /// Bind to specific interface (empty = all)
    pub bind_interface: String,
    
    /// Connection timeout in seconds
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardConfig {
    /// Enable clipboard sharing
    pub enabled: bool,
    
    /// Share text
    pub share_text: bool,
    
    /// Share images
    pub share_images: bool,
    
    /// Share files
    pub share_files: bool,
    
    /// Maximum clipboard size in bytes (default: 10MB)
    pub max_size_bytes: usize,
}

impl Default for Config {
    fn default() -> Self {
        let machine_name = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "MacWinShare".to_string());

        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("MacWinShare");

        Self {
            machine_id: uuid::Uuid::new_v4().to_string(),
            machine_name,
            port: 24800,
            screen: ScreenConfig::default(),
            security: SecurityConfig::default(),
            network: NetworkConfig::default(),
            clipboard: ClipboardConfig::default(),
            data_dir,
        }
    }
}

impl Default for ScreenConfig {
    fn default() -> Self {
        Self {
            position: ScreenPosition { x: 0, y: 0 },
            width: 1920,
            height: 1080,
            edge_threshold: 1,
            neighbors: Vec::new(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            tls_enabled: true,
            verify_clients: true,
            trusted_fingerprints: Vec::new(),
            password: None,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            mdns_enabled: true,
            udp_discovery_enabled: true,
            bind_interface: String::new(),
            timeout_seconds: 30,
        }
    }
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            share_text: true,
            share_images: true,
            share_files: true,
            max_size_bytes: 10 * 1024 * 1024, // 10MB
        }
    }
}

impl Config {
    /// Load configuration from disk or create default
    pub fn load() -> crate::Result<Self> {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("MacWinShare");

        let config_path = data_dir.join("config.json");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = serde_json::from_str(&content)
                .map_err(|e| crate::Error::Config(e.to_string()))?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save configuration to disk
    pub fn save(&self) -> crate::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        let config_path = self.data_dir.join("config.json");
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| crate::Error::Config(e.to_string()))?;
        std::fs::write(config_path, content)?;
        Ok(())
    }
}

// Add dirs dependency for data directory
mod dirs {
    use std::path::PathBuf;

    pub fn data_local_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join("Library/Application Support"))
        }

        #[cfg(target_os = "windows")]
        {
            std::env::var("LOCALAPPDATA")
                .ok()
                .map(PathBuf::from)
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".local/share"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        
        assert!(!config.machine_id.is_empty());
        assert!(!config.machine_name.is_empty());
        assert_eq!(config.port, 24800);
        assert!(config.security.tls_enabled);
        assert!(config.clipboard.enabled);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();
        
        assert_eq!(config.machine_id, deserialized.machine_id);
        assert_eq!(config.machine_name, deserialized.machine_name);
        assert_eq!(config.port, deserialized.port);
    }

    #[test]
    fn test_screen_config_defaults() {
        let screen = ScreenConfig::default();
        
        assert_eq!(screen.position.x, 0);
        assert_eq!(screen.position.y, 0);
        assert_eq!(screen.width, 1920);
        assert_eq!(screen.height, 1080);
        assert_eq!(screen.edge_threshold, 1);
        assert!(screen.neighbors.is_empty());
    }

    #[test]
    fn test_security_config_defaults() {
        let security = SecurityConfig::default();
        
        assert!(security.tls_enabled);
        assert!(security.verify_clients);
        assert!(security.trusted_fingerprints.is_empty());
        assert!(security.password.is_none());
    }

    #[test]
    fn test_network_config_defaults() {
        let network = NetworkConfig::default();
        
        assert!(network.mdns_enabled);
        assert!(network.udp_discovery_enabled);
        assert!(network.bind_interface.is_empty());
        assert_eq!(network.timeout_seconds, 30);
    }

    #[test]
    fn test_clipboard_config_defaults() {
        let clipboard = ClipboardConfig::default();
        
        assert!(clipboard.enabled);
        assert!(clipboard.share_text);
        assert!(clipboard.share_images);
        assert!(clipboard.share_files);
        assert_eq!(clipboard.max_size_bytes, 10 * 1024 * 1024);
    }

    #[test]
    fn test_screen_neighbor_serialization() {
        let neighbor = ScreenNeighbor {
            name: "WindowsPC".to_string(),
            direction: Direction::Right,
        };
        
        let json = serde_json::to_string(&neighbor).unwrap();
        let deserialized: ScreenNeighbor = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.name, "WindowsPC");
        assert_eq!(deserialized.direction, Direction::Right);
    }

    #[test]
    fn test_direction_serialization() {
        for dir in [Direction::Left, Direction::Right, Direction::Top, Direction::Bottom] {
            let json = serde_json::to_string(&dir).unwrap();
            let deserialized: Direction = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, dir);
        }
    }

    #[test]
    fn test_config_with_neighbors() {
        let mut config = Config::default();
        config.screen.neighbors.push(ScreenNeighbor {
            name: "MacBook".to_string(),
            direction: Direction::Left,
        });
        config.screen.neighbors.push(ScreenNeighbor {
            name: "Desktop".to_string(),
            direction: Direction::Right,
        });
        
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.screen.neighbors.len(), 2);
        assert_eq!(deserialized.screen.neighbors[0].name, "MacBook");
        assert_eq!(deserialized.screen.neighbors[1].direction, Direction::Right);
    }

    #[test]
    fn test_config_with_password() {
        let mut config = Config::default();
        config.security.password = Some("secret123".to_string());
        
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.security.password, Some("secret123".to_string()));
    }

    #[test]
    fn test_config_with_fingerprints() {
        let mut config = Config::default();
        config.security.trusted_fingerprints = vec![
            "AA:BB:CC:DD:EE:FF".to_string(),
            "11:22:33:44:55:66".to_string(),
        ];
        
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.security.trusted_fingerprints.len(), 2);
    }

    #[test]
    fn test_config_save_load() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.data_dir = temp_dir.path().to_path_buf();
        config.machine_name = "TestMachine".to_string();
        
        config.save().unwrap();
        
        // Read back the file
        let config_path = temp_dir.path().join("config.json");
        let content = std::fs::read_to_string(&config_path).unwrap();
        let loaded: Config = serde_json::from_str(&content).unwrap();
        
        assert_eq!(loaded.machine_name, "TestMachine");
    }

    #[test]
    fn test_screen_position_equality() {
        let pos1 = ScreenPosition { x: 100, y: 200 };
        let pos2 = ScreenPosition { x: 100, y: 200 };
        let pos3 = ScreenPosition { x: 100, y: 300 };
        
        assert_eq!(pos1, pos2);
        assert_ne!(pos1, pos3);
    }
}
