//! Network discovery using mDNS and UDP broadcast
//! 
//! Allows MacWinShare instances to automatically find each other on the network.

use crate::{Config, Error, Result};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

const SERVICE_TYPE: &str = "_macwinshare._tcp.local.";
const UDP_DISCOVERY_PORT: u16 = 24801;
const DISCOVERY_MAGIC: &[u8] = b"MWSHARE1";

/// Handles network discovery of MacWinShare peers
pub struct Discovery {
    config: Arc<RwLock<Config>>,
    mdns_daemon: Option<ServiceDaemon>,
}

/// Information about a discovered peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub name: String,
    pub address: SocketAddr,
    pub fingerprint: Option<String>,
    pub version: String,
}

impl Discovery {
    /// Create a new Discovery instance
    pub async fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        // Try to create mDNS daemon, but continue without it if unavailable
        let mdns_daemon = match ServiceDaemon::new() {
            Ok(daemon) => Some(daemon),
            Err(e) => {
                warn!("mDNS discovery unavailable: {}", e);
                None
            }
        };

        Ok(Self {
            config,
            mdns_daemon,
        })
    }

    /// Start advertising this machine as a server
    pub async fn start_advertising(&self) -> Result<()> {
        let config = self.config.read().await;
        
        // Advertise via mDNS if available
        if let Some(ref daemon) = self.mdns_daemon {
            let local_ip = local_ip_address::local_ip()
                .map_err(|e| Error::Discovery(e.to_string()))?;

            let mut txt_properties = HashMap::new();
            txt_properties.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
            txt_properties.insert("id".to_string(), config.machine_id.clone());

            let service_info = ServiceInfo::new(
                SERVICE_TYPE,
                &config.machine_name,
                &format!("{}.local.", config.machine_name),
                local_ip,
                config.port,
                Some(txt_properties),
            ).map_err(|e| Error::Discovery(e.to_string()))?;

            daemon
                .register(service_info)
                .map_err(|e| Error::Discovery(e.to_string()))?;

            info!("Advertising service via mDNS: {}", config.machine_name);
        }

        // Also start UDP broadcast listener for fallback discovery
        if config.network.udp_discovery_enabled {
            self.start_udp_responder().await?;
        }

        Ok(())
    }

    /// Find a server on the network
    pub async fn find_server(&self) -> Result<String> {
        let config = self.config.read().await;
        let timeout = Duration::from_secs(config.network.timeout_seconds);

        // Try mDNS first
        if let Some(ref daemon) = self.mdns_daemon {
            if let Some(peer) = self.find_via_mdns(daemon, timeout).await? {
                return Ok(format!("{}:{}", peer.address.ip(), config.port));
            }
        }

        // Fall back to UDP broadcast
        if config.network.udp_discovery_enabled {
            if let Some(peer) = self.find_via_udp_broadcast(timeout).await? {
                return Ok(format!("{}:{}", peer.address.ip(), config.port));
            }
        }

        Err(Error::Discovery("No servers found on network".into()))
    }

    /// Browse for all available peers
    pub async fn browse(&self) -> Result<Vec<PeerInfo>> {
        let mut peers = Vec::new();
        let config = self.config.read().await;
        let timeout = Duration::from_secs(5);

        // Try mDNS
        if let Some(ref daemon) = self.mdns_daemon {
            let browser = daemon
                .browse(SERVICE_TYPE)
                .map_err(|e| Error::Discovery(e.to_string()))?;

            let deadline = std::time::Instant::now() + timeout;
            while std::time::Instant::now() < deadline {
                if let Ok(event) = browser.recv_timeout(Duration::from_millis(100)) {
                    if let ServiceEvent::ServiceResolved(info) = event {
                        if let Some(addr) = info.get_addresses().iter().next() {
                            peers.push(PeerInfo {
                                name: info.get_fullname().to_string(),
                                address: SocketAddr::new(*addr, info.get_port()),
                                fingerprint: info.get_properties().get("fingerprint").map(|v| v.val_str().to_string()),
                                version: info.get_properties().get("version").map(|v| v.val_str().to_string()).unwrap_or_default(),
                            });
                        }
                    }
                }
            }
        }

        // Also try UDP broadcast
        if config.network.udp_discovery_enabled {
            if let Ok(Some(peer)) = self.find_via_udp_broadcast(timeout).await {
                if !peers.iter().any(|p| p.address.ip() == peer.address.ip()) {
                    peers.push(peer);
                }
            }
        }

        Ok(peers)
    }

    async fn find_via_mdns(
        &self,
        daemon: &ServiceDaemon,
        timeout: Duration,
    ) -> Result<Option<PeerInfo>> {
        let browser = daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| Error::Discovery(e.to_string()))?;

        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            match browser.recv_timeout(Duration::from_millis(100)) {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    if let Some(addr) = info.get_addresses().iter().next() {
                        info!("Found server via mDNS: {} at {}", info.get_fullname(), addr);
                        return Ok(Some(PeerInfo {
                            name: info.get_fullname().to_string(),
                            address: SocketAddr::new(*addr, info.get_port()),
                            fingerprint: info.get_properties().get("fingerprint").map(|v| v.val_str().to_string()),
                            version: info.get_properties().get("version").map(|v| v.val_str().to_string()).unwrap_or_default(),
                        }));
                    }
                }
                Ok(_) => continue,
                Err(_) => continue,
            }
        }

        Ok(None)
    }

    async fn find_via_udp_broadcast(&self, timeout: Duration) -> Result<Option<PeerInfo>> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_broadcast(true)?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;

        let config = self.config.read().await;
        
        // Build discovery message
        let mut msg = DISCOVERY_MAGIC.to_vec();
        msg.extend_from_slice(b"QUERY\0");
        msg.extend_from_slice(config.machine_id.as_bytes());

        // Broadcast to common subnet broadcast addresses
        let broadcast_addrs = [
            "255.255.255.255:24801",
            "192.168.1.255:24801",
            "192.168.0.255:24801",
            "10.0.0.255:24801",
        ];

        for addr in &broadcast_addrs {
            if let Ok(addr) = addr.parse::<SocketAddr>() {
                let _ = socket.send_to(&msg, addr);
            }
        }

        debug!("Sent UDP broadcast discovery query");

        // Listen for responses
        let mut buf = [0u8; 1024];
        let deadline = std::time::Instant::now() + timeout;

        while std::time::Instant::now() < deadline {
            match socket.recv_from(&mut buf) {
                Ok((len, addr)) => {
                    if len > DISCOVERY_MAGIC.len() && buf.starts_with(DISCOVERY_MAGIC) {
                        let data = &buf[DISCOVERY_MAGIC.len()..len];
                        if data.starts_with(b"RESPONSE\0") {
                            let payload = &data[9..];
                            if let Ok(name) = std::str::from_utf8(payload) {
                                info!("Found server via UDP: {} at {}", name.trim_end_matches('\0'), addr);
                                return Ok(Some(PeerInfo {
                                    name: name.trim_end_matches('\0').to_string(),
                                    address: addr,
                                    fingerprint: None,
                                    version: String::new(),
                                }));
                            }
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(None)
    }

    async fn start_udp_responder(&self) -> Result<()> {
        let config = self.config.clone();

        tokio::spawn(async move {
            let socket = match UdpSocket::bind(format!("0.0.0.0:{}", UDP_DISCOVERY_PORT)) {
                Ok(s) => s,
                Err(e) => {
                    warn!("Failed to bind UDP discovery socket: {}", e);
                    return;
                }
            };

            socket.set_read_timeout(Some(Duration::from_millis(100))).ok();
            info!("UDP discovery responder listening on port {}", UDP_DISCOVERY_PORT);

            let mut buf = [0u8; 1024];
            loop {
                match socket.recv_from(&mut buf) {
                    Ok((len, addr)) => {
                        if len > DISCOVERY_MAGIC.len() && buf.starts_with(DISCOVERY_MAGIC) {
                            let data = &buf[DISCOVERY_MAGIC.len()..len];
                            if data.starts_with(b"QUERY\0") {
                                // Respond with our info
                                let cfg = config.blocking_read();
                                let mut response = DISCOVERY_MAGIC.to_vec();
                                response.extend_from_slice(b"RESPONSE\0");
                                response.extend_from_slice(cfg.machine_name.as_bytes());
                                
                                let _ = socket.send_to(&response, addr);
                                debug!("Responded to discovery query from {}", addr);
                            }
                        }
                    }
                    Err(_) => continue,
                }
            }
        });

        Ok(())
    }
}
