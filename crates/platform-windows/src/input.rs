//! Windows Input Capture and Injection
//! 
//! Uses low-level keyboard and mouse hooks for capture,
//! and SendInput API for injection.

#[cfg(target_os = "windows")]
use windows::{
    Win32::Foundation::{LPARAM, LRESULT, WPARAM},
    Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
        KEYEVENTF_SCANCODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
        MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT,
    },
    Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetCursorPos, SetCursorPos, SetWindowsHookExW, UnhookWindowsHookEx,
        HHOOK, KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN,
        WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE,
        WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    },
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, info};

/// Windows input handler
pub struct WindowsInput {
    capturing: Arc<AtomicBool>,
    #[cfg(target_os = "windows")]
    keyboard_hook: Option<HHOOK>,
    #[cfg(target_os = "windows")]
    mouse_hook: Option<HHOOK>,
}

impl WindowsInput {
    pub fn new() -> Self {
        Self {
            capturing: Arc::new(AtomicBool::new(false)),
            #[cfg(target_os = "windows")]
            keyboard_hook: None,
            #[cfg(target_os = "windows")]
            mouse_hook: None,
        }
    }

    /// Start capturing input events
    #[cfg(target_os = "windows")]
    pub fn start_capture(&mut self) -> Result<(), String> {
        if self.capturing.load(Ordering::SeqCst) {
            return Err("Already capturing".into());
        }

        unsafe {
            // Install keyboard hook
            let kb_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)
                .map_err(|e| format!("Failed to install keyboard hook: {}", e))?;
            self.keyboard_hook = Some(kb_hook);

            // Install mouse hook
            let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0)
                .map_err(|e| format!("Failed to install mouse hook: {}", e))?;
            self.mouse_hook = Some(mouse_hook);
        }

        self.capturing.store(true, Ordering::SeqCst);
        info!("Windows input capture started");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn start_capture(&mut self) -> Result<(), String> {
        Err("Windows input capture only available on Windows".into())
    }

    /// Stop capturing input events
    #[cfg(target_os = "windows")]
    pub fn stop_capture(&mut self) {
        unsafe {
            if let Some(hook) = self.keyboard_hook.take() {
                let _ = UnhookWindowsHookEx(hook);
            }
            if let Some(hook) = self.mouse_hook.take() {
                let _ = UnhookWindowsHookEx(hook);
            }
        }
        self.capturing.store(false, Ordering::SeqCst);
        info!("Windows input capture stopped");
    }

    #[cfg(not(target_os = "windows"))]
    pub fn stop_capture(&mut self) {
        self.capturing.store(false, Ordering::SeqCst);
    }

    /// Move the mouse cursor to an absolute position
    #[cfg(target_os = "windows")]
    pub fn move_cursor(&self, x: i32, y: i32) -> Result<(), String> {
        unsafe {
            SetCursorPos(x, y).map_err(|e| format!("Failed to set cursor position: {}", e))?;
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn move_cursor(&self, _x: i32, _y: i32) -> Result<(), String> {
        Err("Windows input only available on Windows".into())
    }

    /// Get the current cursor position
    #[cfg(target_os = "windows")]
    pub fn get_cursor_position(&self) -> Result<(i32, i32), String> {
        unsafe {
            let mut point = windows::Win32::Foundation::POINT::default();
            GetCursorPos(&mut point).map_err(|e| format!("Failed to get cursor position: {}", e))?;
            Ok((point.x, point.y))
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn get_cursor_position(&self) -> Result<(i32, i32), String> {
        Err("Windows input only available on Windows".into())
    }

    /// Inject a mouse button event
    #[cfg(target_os = "windows")]
    pub fn mouse_button(&self, button: MouseButton, pressed: bool) -> Result<(), String> {
        let flags = match (button, pressed) {
            (MouseButton::Left, true) => MOUSEEVENTF_LEFTDOWN,
            (MouseButton::Left, false) => MOUSEEVENTF_LEFTUP,
            (MouseButton::Right, true) => MOUSEEVENTF_RIGHTDOWN,
            (MouseButton::Right, false) => MOUSEEVENTF_RIGHTUP,
            (MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEDOWN,
            (MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEUP,
        };

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn mouse_button(&self, _button: MouseButton, _pressed: bool) -> Result<(), String> {
        Err("Windows input only available on Windows".into())
    }

    /// Inject a scroll wheel event
    #[cfg(target_os = "windows")]
    pub fn scroll(&self, delta_x: i32, delta_y: i32) -> Result<(), String> {
        // Vertical scroll
        if delta_y != 0 {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: (delta_y * 120) as u32, // WHEEL_DELTA = 120
                        dwFlags: MOUSEEVENTF_WHEEL,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };

            unsafe {
                SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            }
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn scroll(&self, _delta_x: i32, _delta_y: i32) -> Result<(), String> {
        Err("Windows input only available on Windows".into())
    }

    /// Inject a keyboard event
    #[cfg(target_os = "windows")]
    pub fn key_event(&self, vk_code: u16, scan_code: u16, pressed: bool) -> Result<(), String> {
        let mut flags = KEYEVENTF_SCANCODE;
        if !pressed {
            flags |= KEYEVENTF_KEYUP;
        }

        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk_code),
                    wScan: scan_code,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn key_event(&self, _vk_code: u16, _scan_code: u16, _pressed: bool) -> Result<(), String> {
        Err("Windows input only available on Windows".into())
    }
}

impl Default for WindowsInput {
    fn default() -> Self {
        Self::new()
    }
}

/// Input events that can be captured
#[derive(Debug, Clone)]
pub enum InputEvent {
    MouseMove { x: i32, y: i32 },
    MouseDown { button: MouseButton },
    MouseUp { button: MouseButton },
    Scroll { delta: i32 },
    KeyDown { vk_code: u16, scan_code: u16 },
    KeyUp { vk_code: u16, scan_code: u16 },
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

// Hook procedures (Windows-only)
#[cfg(target_os = "windows")]
unsafe extern "system" fn keyboard_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let kb_struct = *(lparam.0 as *const KBDLLHOOKSTRUCT);
        let _vk_code = kb_struct.vkCode as u16;
        let _scan_code = kb_struct.scanCode as u16;

        match wparam.0 as u32 {
            x if x == WM_KEYDOWN || x == WM_SYSKEYDOWN => {
                // Key down event
                debug!("Key down: vk={}, scan={}", _vk_code, _scan_code);
            }
            x if x == WM_KEYUP || x == WM_SYSKEYUP => {
                // Key up event
                debug!("Key up: vk={}, scan={}", _vk_code, _scan_code);
            }
            _ => {}
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn mouse_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let mouse_struct = *(lparam.0 as *const MSLLHOOKSTRUCT);
        let _x = mouse_struct.pt.x;
        let _y = mouse_struct.pt.y;

        match wparam.0 as u32 {
            WM_MOUSEMOVE => {
                debug!("Mouse move: ({}, {})", _x, _y);
            }
            WM_LBUTTONDOWN => {
                debug!("Left button down");
            }
            WM_LBUTTONUP => {
                debug!("Left button up");
            }
            WM_RBUTTONDOWN => {
                debug!("Right button down");
            }
            WM_RBUTTONUP => {
                debug!("Right button up");
            }
            WM_MBUTTONDOWN => {
                debug!("Middle button down");
            }
            WM_MBUTTONUP => {
                debug!("Middle button up");
            }
            WM_MOUSEWHEEL => {
                let _delta = (mouse_struct.mouseData >> 16) as i16;
                debug!("Mouse wheel: {}", _delta);
            }
            _ => {}
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}

/// Convert Windows virtual key code to Synergy/Deskflow keycode
pub fn windows_to_synergy_keycode(vk_code: u16) -> u16 {
    match vk_code {
        // Letters (VK_A through VK_Z are 0x41-0x5A)
        0x41..=0x5A => 0x0061 + (vk_code - 0x41), // Convert to lowercase ASCII
        
        // Numbers (VK_0 through VK_9 are 0x30-0x39)
        0x30..=0x39 => vk_code, // Same as ASCII
        
        // Function keys
        0x70 => 0xFFBE, // F1
        0x71 => 0xFFBF, // F2
        0x72 => 0xFFC0, // F3
        0x73 => 0xFFC1, // F4
        0x74 => 0xFFC2, // F5
        0x75 => 0xFFC3, // F6
        0x76 => 0xFFC4, // F7
        0x77 => 0xFFC5, // F8
        0x78 => 0xFFC6, // F9
        0x79 => 0xFFC7, // F10
        0x7A => 0xFFC8, // F11
        0x7B => 0xFFC9, // F12
        
        // Modifiers
        0x10 => 0xFFE1, // VK_SHIFT
        0x11 => 0xFFE3, // VK_CONTROL
        0x12 => 0xFFE9, // VK_MENU (Alt)
        0x5B => 0xFFEB, // VK_LWIN
        0x5C => 0xFFEC, // VK_RWIN
        
        // Special keys
        0x0D => 0xFF0D, // VK_RETURN
        0x09 => 0xFF09, // VK_TAB
        0x20 => 0x0020, // VK_SPACE
        0x08 => 0xFF08, // VK_BACK
        0x2E => 0xFFFF, // VK_DELETE
        0x2D => 0xFF63, // VK_INSERT
        0x24 => 0xFF50, // VK_HOME
        0x23 => 0xFF57, // VK_END
        0x21 => 0xFF55, // VK_PRIOR (Page Up)
        0x22 => 0xFF56, // VK_NEXT (Page Down)
        0x1B => 0xFF1B, // VK_ESCAPE
        
        // Arrow keys
        0x25 => 0xFF51, // VK_LEFT
        0x26 => 0xFF52, // VK_UP
        0x27 => 0xFF53, // VK_RIGHT
        0x28 => 0xFF54, // VK_DOWN
        
        // Locks
        0x14 => 0xFFE5, // VK_CAPITAL (Caps Lock)
        0x90 => 0xFF7F, // VK_NUMLOCK
        0x91 => 0xFF14, // VK_SCROLL
        
        // Default: pass through
        _ => vk_code,
    }
}
