//! Platform abstraction layer
//! 
//! Provides a common interface for platform-specific input capture and injection.

use async_trait::async_trait;
use crate::Result;
use crate::screen::CursorPosition;

/// Input events that can be captured or injected
#[derive(Debug, Clone)]
pub enum InputEvent {
    // Keyboard events
    KeyDown {
        key_code: u16,
        modifiers: u16,
        scan_code: u16,
    },
    KeyUp {
        key_code: u16,
        modifiers: u16,
        scan_code: u16,
    },
    
    // Mouse events
    MouseMove {
        x: i32,
        y: i32,
        absolute: bool,
    },
    MouseDown {
        button: MouseButton,
    },
    MouseUp {
        button: MouseButton,
    },
    MouseWheel {
        delta_x: i32,
        delta_y: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

/// Trait for platform-specific input handling
#[async_trait]
pub trait PlatformInput: Send + Sync {
    /// Start capturing input events
    async fn start_capture(&mut self) -> Result<()>;
    
    /// Stop capturing input events
    async fn stop_capture(&mut self) -> Result<()>;
    
    /// Inject an input event
    async fn inject_event(&self, event: InputEvent) -> Result<()>;
    
    /// Get the current cursor position
    fn get_cursor_position(&self) -> Result<CursorPosition>;
    
    /// Set the cursor position
    fn set_cursor_position(&self, pos: CursorPosition) -> Result<()>;
    
    /// Hide the cursor
    fn hide_cursor(&self) -> Result<()>;
    
    /// Show the cursor
    fn show_cursor(&self) -> Result<()>;
    
    /// Check if accessibility permissions are granted (macOS)
    fn check_accessibility_permission(&self) -> bool {
        true // Default to true for platforms that don't need it
    }
    
    /// Request accessibility permissions (macOS)
    fn request_accessibility_permission(&self) -> Result<()> {
        Ok(()) // No-op for platforms that don't need it
    }
}

/// Trait for platform-specific display information
pub trait PlatformDisplay: Send + Sync {
    /// Get all connected displays
    fn get_displays(&self) -> Result<Vec<DisplayInfo>>;
    
    /// Get the primary display
    fn get_primary_display(&self) -> Result<DisplayInfo>;
    
    /// Get the display containing a point
    fn get_display_at(&self, x: i32, y: i32) -> Result<Option<DisplayInfo>>;
}

/// Information about a display
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub scale_factor: f64,
    pub is_primary: bool,
}

// Re-export platform-specific implementations
#[cfg(target_os = "macos")]
pub use macwinshare_platform_macos as native;

#[cfg(target_os = "windows")]
pub use macwinshare_platform_windows as native;

// Stub for unsupported platforms
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub mod native {
    use super::*;
    
    pub struct PlatformInputImpl;
    
    #[async_trait]
    impl PlatformInput for PlatformInputImpl {
        async fn start_capture(&mut self) -> Result<()> {
            Err(crate::Error::Platform("Unsupported platform".into()))
        }
        async fn stop_capture(&mut self) -> Result<()> { Ok(()) }
        async fn inject_event(&self, _: InputEvent) -> Result<()> {
            Err(crate::Error::Platform("Unsupported platform".into()))
        }
        fn get_cursor_position(&self) -> Result<CursorPosition> {
            Ok(CursorPosition::new(0, 0))
        }
        fn set_cursor_position(&self, _: CursorPosition) -> Result<()> { Ok(()) }
        fn hide_cursor(&self) -> Result<()> { Ok(()) }
        fn show_cursor(&self) -> Result<()> { Ok(()) }
    }
    
    pub struct PlatformDisplayImpl;
    
    impl PlatformDisplay for PlatformDisplayImpl {
        fn get_displays(&self) -> Result<Vec<DisplayInfo>> { Ok(vec![]) }
        fn get_primary_display(&self) -> Result<DisplayInfo> {
            Err(crate::Error::Platform("Unsupported platform".into()))
        }
        fn get_display_at(&self, _: i32, _: i32) -> Result<Option<DisplayInfo>> { Ok(None) }
    }
}
