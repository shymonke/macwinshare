//! macOS Display Information
//! 
//! Uses CoreGraphics to get information about connected displays.

use core_graphics::display::{CGDisplay, CGDisplayBounds};
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

/// macOS display handler
pub struct MacOSDisplay;

impl MacOSDisplay {
    pub fn new() -> Self {
        Self
    }

    /// Get all connected displays
    pub fn get_displays(&self) -> Result<Vec<DisplayInfo>, String> {
        let active_displays = CGDisplay::active_displays()
            .map_err(|_| "Failed to get active displays")?;

        let main_display = CGDisplay::main();
        let main_id = main_display.id;

        let mut displays = Vec::new();

        for display_id in active_displays {
            let display = CGDisplay::new(display_id);
            let bounds = display.bounds();

            displays.push(DisplayInfo {
                id: display_id,
                name: format!("Display {}", display_id),
                x: bounds.origin.x as i32,
                y: bounds.origin.y as i32,
                width: bounds.size.width as i32,
                height: bounds.size.height as i32,
                scale_factor: Self::get_scale_factor(display_id),
                is_primary: display_id == main_id,
            });

            debug!(
                "Found display {}: {}x{} at ({}, {})",
                display_id,
                bounds.size.width,
                bounds.size.height,
                bounds.origin.x,
                bounds.origin.y
            );
        }

        Ok(displays)
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

    fn get_scale_factor(display_id: u32) -> f64 {
        // On macOS, we can get the scale factor from the display mode
        let display = CGDisplay::new(display_id);
        
        // Get the display's pixel dimensions vs point dimensions
        let bounds = display.bounds();
        let pixels_wide = display.pixels_wide() as f64;
        
        if bounds.size.width > 0.0 {
            pixels_wide / bounds.size.width
        } else {
            1.0
        }
    }
}

impl Default for MacOSDisplay {
    fn default() -> Self {
        Self::new()
    }
}
