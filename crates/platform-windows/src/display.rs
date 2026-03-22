//! Windows Display Information
//! 
//! Uses Win32 API to get information about connected monitors.

#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{LPARAM, RECT};
#[cfg(target_os = "windows")]
use windows::core::BOOL;
use std::sync::Mutex;
use tracing::debug;

/// Display information
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

/// Windows display handler
pub struct WindowsDisplay {
    #[cfg(target_os = "windows")]
    displays: Mutex<Vec<DisplayInfo>>,
}

impl WindowsDisplay {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            displays: Mutex::new(Vec::new()),
        }
    }

    /// Get all connected displays
    #[cfg(target_os = "windows")]
    pub fn get_displays(&self) -> Result<Vec<DisplayInfo>, String> {
        let mut displays = self.displays.lock().unwrap();
        displays.clear();

        unsafe {
            EnumDisplayMonitors(
                None,
                None,
                Some(monitor_enum_proc),
                LPARAM(&mut *displays as *mut Vec<DisplayInfo> as isize),
            )
            .ok()
            .map_err(|e| format!("Failed to enumerate monitors: {}", e))?;
        }

        Ok(displays.clone())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn get_displays(&self) -> Result<Vec<DisplayInfo>, String> {
        Err("Windows display only available on Windows".into())
    }

    /// Get the primary display
    pub fn get_primary_display(&self) -> Result<DisplayInfo, String> {
        let displays = self.get_displays()?;
        displays
            .into_iter()
            .find(|d| d.is_primary)
            .ok_or_else(|| "No primary display found".into())
    }

    /// Get the display at a specific point
    pub fn get_display_at(&self, x: i32, y: i32) -> Result<Option<DisplayInfo>, String> {
        let displays = self.get_displays()?;
        
        Ok(displays.into_iter().find(|d| {
            x >= d.x && x < d.x + d.width && y >= d.y && y < d.y + d.height
        }))
    }

    /// Get total virtual screen bounds (all displays combined)
    pub fn get_virtual_screen_bounds(&self) -> Result<(i32, i32, i32, i32), String> {
        let displays = self.get_displays()?;
        
        if displays.is_empty() {
            return Err("No displays found".into());
        }

        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for d in &displays {
            min_x = min_x.min(d.x);
            min_y = min_y.min(d.y);
            max_x = max_x.max(d.x + d.width);
            max_y = max_y.max(d.y + d.height);
        }

        Ok((min_x, min_y, max_x - min_x, max_y - min_y))
    }
}

impl Default for WindowsDisplay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn monitor_enum_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let displays = &mut *(lparam.0 as *mut Vec<DisplayInfo>);

    let mut monitor_info = MONITORINFOEXW::default();
    monitor_info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    if GetMonitorInfoW(hmonitor, &mut monitor_info.monitorInfo as *mut _ as *mut _).as_bool() {
        let rect = monitor_info.monitorInfo.rcMonitor;
        let is_primary = (monitor_info.monitorInfo.dwFlags & 1) != 0; // MONITORINFOF_PRIMARY = 1

        // Convert device name from wide string
        let name: String = monitor_info
            .szDevice
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| char::from_u32(c as u32).unwrap_or('?'))
            .collect();

        displays.push(DisplayInfo {
            id: displays.len() as u32,
            name,
            x: rect.left,
            y: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
            scale_factor: 1.0, // Would need DPI awareness APIs for accurate scale
            is_primary,
        });

        debug!(
            "Found monitor: {}x{} at ({}, {}), primary={}",
            rect.right - rect.left,
            rect.bottom - rect.top,
            rect.left,
            rect.top,
            is_primary
        );
    }

    BOOL::from(true) // Continue enumeration
}
