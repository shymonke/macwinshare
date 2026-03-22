//! macOS Input Capture and Injection
//! 
//! Uses CGEventTap for capturing and CGEventPost for injecting input events.

use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, info};

/// macOS input handler
pub struct MacOSInput {
    capturing: Arc<AtomicBool>,
}

impl MacOSInput {
    pub fn new() -> Self {
        Self {
            capturing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start capturing input events
    pub fn start_capture<F>(&self, _callback: F) -> Result<(), String>
    where
        F: Fn(InputEvent) + Send + 'static,
    {
        if self.capturing.load(Ordering::SeqCst) {
            return Err("Already capturing".into());
        }

        // Check accessibility permissions first
        if !super::accessibility::check_accessibility() {
            return Err("Accessibility permission required. Please enable in System Settings > Privacy & Security > Accessibility".into());
        }

        self.capturing.store(true, Ordering::SeqCst);
        
        // In a full implementation, we would create an event tap here
        // For now, we'll log that capture started
        info!("macOS input capture started");

        Ok(())
    }

    /// Stop capturing input events
    pub fn stop_capture(&self) {
        self.capturing.store(false, Ordering::SeqCst);
        info!("macOS input capture stopped");
    }

    /// Move the mouse cursor to an absolute position
    pub fn move_cursor(&self, x: f64, y: f64) -> Result<(), String> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| "Failed to create event source")?;

        let event = CGEvent::new_mouse_event(
            source,
            CGEventType::MouseMoved,
            CGPoint::new(x, y),
            CGMouseButton::Left,
        )
        .map_err(|_| "Failed to create mouse event")?;

        event.post(CGEventTapLocation::HID);
        Ok(())
    }

    /// Inject a mouse button event
    pub fn mouse_button(&self, button: MouseButton, pressed: bool) -> Result<(), String> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| "Failed to create event source")?;

        let (cg_button, down_type, up_type) = match button {
            MouseButton::Left => (
                CGMouseButton::Left,
                CGEventType::LeftMouseDown,
                CGEventType::LeftMouseUp,
            ),
            MouseButton::Right => (
                CGMouseButton::Right,
                CGEventType::RightMouseDown,
                CGEventType::RightMouseUp,
            ),
            MouseButton::Middle => (
                CGMouseButton::Center,
                CGEventType::OtherMouseDown,
                CGEventType::OtherMouseUp,
            ),
        };

        let event_type = if pressed { down_type } else { up_type };
        let pos = Self::get_cursor_position_internal()?;

        let event = CGEvent::new_mouse_event(source, event_type, pos, cg_button)
            .map_err(|_| "Failed to create mouse button event")?;

        event.post(CGEventTapLocation::HID);
        Ok(())
    }

    /// Inject a scroll wheel event using CGEventSetIntegerValueField
    pub fn scroll(&self, _delta_x: i32, delta_y: i32) -> Result<(), String> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| "Failed to create event source")?;

        // Create a generic event and set scroll wheel fields
        let event = CGEvent::new(source)
            .map_err(|_| "Failed to create event")?;
        
        event.set_type(CGEventType::ScrollWheel);
        // Set scroll wheel delta using field 11 (kCGScrollWheelEventDeltaAxis1)
        event.set_integer_value_field(11, delta_y as i64);
        
        event.post(CGEventTapLocation::HID);
        Ok(())
    }

    /// Inject a keyboard event
    pub fn key_event(&self, keycode: u16, pressed: bool) -> Result<(), String> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| "Failed to create event source")?;

        let event = CGEvent::new_keyboard_event(source, keycode, pressed)
            .map_err(|_| "Failed to create keyboard event")?;

        event.post(CGEventTapLocation::HID);
        Ok(())
    }

    /// Get the current cursor position
    pub fn get_cursor_position(&self) -> Result<(f64, f64), String> {
        let pos = Self::get_cursor_position_internal()?;
        Ok((pos.x, pos.y))
    }

    fn get_cursor_position_internal() -> Result<CGPoint, String> {
        // Use CGEvent to get the current mouse location
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| "Failed to create event source")?;
        
        let event = CGEvent::new(source)
            .map_err(|_| "Failed to create event")?;
        
        Ok(event.location())
    }

    /// Hide the cursor
    pub fn hide_cursor(&self) -> Result<(), String> {
        // Note: This requires AppKit, which we're not directly using
        // In a full implementation, we'd use NSCursor.hide()
        debug!("hide_cursor called (stub)");
        Ok(())
    }

    /// Show the cursor
    pub fn show_cursor(&self) -> Result<(), String> {
        debug!("show_cursor called (stub)");
        Ok(())
    }
}

impl Default for MacOSInput {
    fn default() -> Self {
        Self::new()
    }
}

/// Input events that can be captured
#[derive(Debug, Clone)]
pub enum InputEvent {
    MouseMove { x: f64, y: f64 },
    MouseDown { button: MouseButton },
    MouseUp { button: MouseButton },
    Scroll { delta_x: i32, delta_y: i32 },
    KeyDown { keycode: u16, flags: u64 },
    KeyUp { keycode: u16, flags: u64 },
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Convert macOS keycode to Synergy/Deskflow keycode
pub fn macos_to_synergy_keycode(keycode: u16) -> u16 {
    // macOS keycodes are different from X11/Synergy keycodes
    // This is a basic mapping - a full implementation would have a complete table
    match keycode {
        // Letters (macOS uses physical key positions)
        0x00 => 0x0061, // A
        0x01 => 0x0073, // S
        0x02 => 0x0064, // D
        0x03 => 0x0066, // F
        0x04 => 0x0068, // H
        0x05 => 0x0067, // G
        0x06 => 0x007A, // Z
        0x07 => 0x0078, // X
        0x08 => 0x0063, // C
        0x09 => 0x0076, // V
        0x0B => 0x0062, // B
        0x0C => 0x0071, // Q
        0x0D => 0x0077, // W
        0x0E => 0x0065, // E
        0x0F => 0x0072, // R
        0x10 => 0x0079, // Y
        0x11 => 0x0074, // T
        
        // Numbers
        0x12 => 0x0031, // 1
        0x13 => 0x0032, // 2
        0x14 => 0x0033, // 3
        0x15 => 0x0034, // 4
        0x16 => 0x0036, // 6
        0x17 => 0x0035, // 5
        0x19 => 0x0039, // 9
        0x1A => 0x0037, // 7
        0x1C => 0x0038, // 8
        0x1D => 0x0030, // 0
        
        // Special keys
        0x24 => 0xFF0D, // Return
        0x30 => 0xFF09, // Tab
        0x31 => 0x0020, // Space
        0x33 => 0xFF08, // Backspace
        0x35 => 0xFF1B, // Escape
        
        // Modifiers
        0x37 => 0xFFE7, // Command (Left)
        0x38 => 0xFFE1, // Shift (Left)
        0x3A => 0xFFE9, // Option/Alt (Left)
        0x3B => 0xFFE3, // Control (Left)
        0x36 => 0xFFE8, // Command (Right)
        0x3C => 0xFFE2, // Shift (Right)
        0x3D => 0xFFEA, // Option/Alt (Right)
        0x3E => 0xFFE4, // Control (Right)
        
        // Arrow keys
        0x7B => 0xFF51, // Left
        0x7C => 0xFF53, // Right
        0x7D => 0xFF54, // Down
        0x7E => 0xFF52, // Up
        
        // Function keys
        0x7A => 0xFFBE, // F1
        0x78 => 0xFFBF, // F2
        0x63 => 0xFFC0, // F3
        0x76 => 0xFFC1, // F4
        0x60 => 0xFFC2, // F5
        0x61 => 0xFFC3, // F6
        0x62 => 0xFFC4, // F7
        0x64 => 0xFFC5, // F8
        0x65 => 0xFFC6, // F9
        0x6D => 0xFFC7, // F10
        0x67 => 0xFFC8, // F11
        0x6F => 0xFFC9, // F12
        
        // Default: pass through
        _ => keycode,
    }
}
