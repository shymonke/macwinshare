//! Tauri IPC Commands

use macwinshare_core::{Config, Discovery};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

type ConfigState = Arc<RwLock<Config>>;

#[derive(serde::Serialize)]
pub struct AppStatus {
    pub mode: String,
    pub connected: bool,
    pub peer_name: Option<String>,
    pub fingerprint: String,
}

#[derive(serde::Serialize)]
pub struct PeerInfo {
    pub name: String,
    pub address: String,
    pub fingerprint: Option<String>,
}

#[derive(serde::Serialize)]
pub struct DisplayInfo {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub is_primary: bool,
}

/// Get the current configuration
#[tauri::command]
pub async fn get_config(config: State<'_, ConfigState>) -> Result<Config, String> {
    let cfg = config.read().await;
    Ok(cfg.clone())
}

/// Save the configuration
#[tauri::command]
pub async fn save_config(
    config: State<'_, ConfigState>,
    new_config: Config,
) -> Result<(), String> {
    let mut cfg = config.write().await;
    *cfg = new_config;
    cfg.save().map_err(|e| e.to_string())
}

/// Start the server
#[tauri::command]
pub async fn start_server(config: State<'_, ConfigState>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if !macwinshare_platform_macos::check_accessibility() {
            macwinshare_platform_macos::accessibility::request_accessibility();
            return Err("Accessibility permission required. macOS Settings should open; enable MacWinShare under Privacy & Security > Accessibility, then try again.".to_string());
        }
    }

    let cfg = config.read().await.clone();
    
    // Start server in background
    tokio::spawn(async move {
        match macwinshare_core::MacWinShare::new(cfg).await {
            Ok(mut app) => {
                if let Err(e) = app.start_server().await {
                    tracing::error!("Server error: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to create server: {}", e);
            }
        }
    });

    Ok(())
}

/// Start the client
#[tauri::command]
pub async fn start_client(
    config: State<'_, ConfigState>,
    server_addr: Option<String>,
) -> Result<(), String> {
    let cfg = config.read().await.clone();
    
    tokio::spawn(async move {
        match macwinshare_core::MacWinShare::new(cfg).await {
            Ok(mut app) => {
                if let Err(e) = app.start_client(server_addr).await {
                    tracing::error!("Client error: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to create client: {}", e);
            }
        }
    });

    Ok(())
}

/// Stop the application
#[tauri::command]
pub async fn stop() -> Result<(), String> {
    // In a full implementation, this would signal the running server/client to stop
    tracing::info!("Stop requested");
    Ok(())
}

/// Get the current status
#[tauri::command]
pub async fn get_status(config: State<'_, ConfigState>) -> Result<AppStatus, String> {
    let cfg = config.read().await;
    
    // Create TLS manager to get fingerprint
    let fingerprint = match macwinshare_core::TlsManager::new(&cfg) {
        Ok(tls) => tls.fingerprint().to_string(),
        Err(_) => "Unknown".to_string(),
    };

    Ok(AppStatus {
        mode: "Idle".to_string(),
        connected: false,
        peer_name: None,
        fingerprint,
    })
}

/// Discover peers on the network
#[tauri::command]
pub async fn discover_peers(config: State<'_, ConfigState>) -> Result<Vec<PeerInfo>, String> {
    let cfg = config.read().await.clone();
    let config_arc = Arc::new(RwLock::new(cfg));
    
    let discovery = Discovery::new(config_arc)
        .await
        .map_err(|e| e.to_string())?;

    let peers = discovery.browse().await.map_err(|e| e.to_string())?;

    Ok(peers
        .into_iter()
        .map(|p| PeerInfo {
            name: p.name,
            address: p.address.to_string(),
            fingerprint: p.fingerprint,
        })
        .collect())
}

/// Get display information
#[tauri::command]
pub async fn get_displays() -> Result<Vec<DisplayInfo>, String> {
    #[cfg(target_os = "macos")]
    {
        use macwinshare_platform_macos::MacOSDisplay;
        let display = MacOSDisplay::new();
        let displays = display.get_displays().map_err(|e| e.to_string())?;
        
        Ok(displays
            .into_iter()
            .map(|d| DisplayInfo {
                id: d.id,
                name: d.name,
                x: d.x,
                y: d.y,
                width: d.width,
                height: d.height,
                is_primary: d.is_primary,
            })
            .collect())
    }

    #[cfg(target_os = "windows")]
    {
        use macwinshare_platform_windows::WindowsDisplay;
        let display = WindowsDisplay::new();
        let displays = display.get_displays().map_err(|e| e.to_string())?;
        
        Ok(displays
            .into_iter()
            .map(|d| DisplayInfo {
                id: d.id,
                name: d.name,
                x: d.x,
                y: d.y,
                width: d.width,
                height: d.height,
                is_primary: d.is_primary,
            })
            .collect())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Unsupported platform".to_string())
    }
}

/// Check accessibility permissions (macOS only)
#[tauri::command]
pub async fn check_accessibility() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        Ok(macwinshare_platform_macos::check_accessibility())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(true) // Always true on Windows (no accessibility permission needed)
    }
}
