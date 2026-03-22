//! Screen edge detection and cursor management

use crate::config::{Direction, ScreenConfig, ScreenNeighbor};
use serde::{Deserialize, Serialize};

/// Represents a cursor position
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CursorPosition {
    pub x: i32,
    pub y: i32,
}

impl CursorPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Screen edge detector
pub struct ScreenEdgeDetector {
    config: ScreenConfig,
}

impl ScreenEdgeDetector {
    pub fn new(config: ScreenConfig) -> Self {
        Self { config }
    }

    /// Update the screen configuration
    pub fn update_config(&mut self, config: ScreenConfig) {
        self.config = config;
    }

    /// Check if cursor is at a screen edge and should switch to another screen
    /// Returns the direction and relative position if switching should occur
    pub fn check_edge(&self, pos: CursorPosition) -> Option<(Direction, CursorPosition)> {
        let threshold = self.config.edge_threshold;

        // Check left edge
        if pos.x <= threshold {
            if self.has_neighbor(Direction::Left) {
                return Some((Direction::Left, CursorPosition::new(
                    self.config.width - threshold - 1,
                    pos.y,
                )));
            }
        }

        // Check right edge
        if pos.x >= self.config.width - threshold {
            if self.has_neighbor(Direction::Right) {
                return Some((Direction::Right, CursorPosition::new(
                    threshold + 1,
                    pos.y,
                )));
            }
        }

        // Check top edge
        if pos.y <= threshold {
            if self.has_neighbor(Direction::Top) {
                return Some((Direction::Top, CursorPosition::new(
                    pos.x,
                    self.config.height - threshold - 1,
                )));
            }
        }

        // Check bottom edge
        if pos.y >= self.config.height - threshold {
            if self.has_neighbor(Direction::Bottom) {
                return Some((Direction::Bottom, CursorPosition::new(
                    pos.x,
                    threshold + 1,
                )));
            }
        }

        None
    }

    /// Get the neighbor in a specific direction
    pub fn get_neighbor(&self, direction: Direction) -> Option<&ScreenNeighbor> {
        self.config.neighbors.iter().find(|n| n.direction == direction)
    }

    /// Check if there's a neighbor in a specific direction
    pub fn has_neighbor(&self, direction: Direction) -> bool {
        self.config.neighbors.iter().any(|n| n.direction == direction)
    }

    /// Calculate the entry position when entering this screen from a direction
    pub fn calculate_entry_position(&self, from_direction: Direction, relative_pos: CursorPosition) -> CursorPosition {
        let threshold = self.config.edge_threshold;

        match from_direction {
            Direction::Left => {
                // Entering from left means we're coming from the left screen
                CursorPosition::new(threshold + 1, relative_pos.y.clamp(0, self.config.height - 1))
            }
            Direction::Right => {
                // Entering from right
                CursorPosition::new(self.config.width - threshold - 1, relative_pos.y.clamp(0, self.config.height - 1))
            }
            Direction::Top => {
                // Entering from top
                CursorPosition::new(relative_pos.x.clamp(0, self.config.width - 1), threshold + 1)
            }
            Direction::Bottom => {
                // Entering from bottom
                CursorPosition::new(relative_pos.x.clamp(0, self.config.width - 1), self.config.height - threshold - 1)
            }
        }
    }

    /// Get screen dimensions
    pub fn dimensions(&self) -> (i32, i32) {
        (self.config.width, self.config.height)
    }
}

/// Manages multiple screens in a virtual desktop layout
pub struct ScreenLayout {
    screens: Vec<ScreenInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenInfo {
    pub name: String,
    pub position: (i32, i32),
    pub width: i32,
    pub height: i32,
}

impl ScreenLayout {
    pub fn new() -> Self {
        Self { screens: Vec::new() }
    }

    pub fn add_screen(&mut self, info: ScreenInfo) {
        self.screens.push(info);
    }

    pub fn remove_screen(&mut self, name: &str) {
        self.screens.retain(|s| s.name != name);
    }

    pub fn get_screen(&self, name: &str) -> Option<&ScreenInfo> {
        self.screens.iter().find(|s| s.name == name)
    }

    pub fn screens(&self) -> &[ScreenInfo] {
        &self.screens
    }

    /// Determine which screen a global position belongs to
    pub fn screen_at_position(&self, x: i32, y: i32) -> Option<&ScreenInfo> {
        self.screens.iter().find(|s| {
            x >= s.position.0 
                && x < s.position.0 + s.width
                && y >= s.position.1 
                && y < s.position.1 + s.height
        })
    }

    /// Convert global position to screen-local position
    pub fn global_to_local(&self, screen: &ScreenInfo, x: i32, y: i32) -> (i32, i32) {
        (x - screen.position.0, y - screen.position.1)
    }

    /// Convert screen-local position to global position
    pub fn local_to_global(&self, screen: &ScreenInfo, x: i32, y: i32) -> (i32, i32) {
        (x + screen.position.0, y + screen.position.1)
    }
}

impl Default for ScreenLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_detection() {
        let config = ScreenConfig {
            position: crate::config::ScreenPosition { x: 0, y: 0 },
            width: 1920,
            height: 1080,
            edge_threshold: 1,
            neighbors: vec![
                ScreenNeighbor { name: "right-screen".into(), direction: Direction::Right },
            ],
        };

        let detector = ScreenEdgeDetector::new(config);

        // Test right edge
        let result = detector.check_edge(CursorPosition::new(1919, 500));
        assert!(result.is_some());
        let (direction, _) = result.unwrap();
        assert_eq!(direction, Direction::Right);

        // Test center (no edge)
        let result = detector.check_edge(CursorPosition::new(960, 540));
        assert!(result.is_none());

        // Test left edge (no neighbor)
        let result = detector.check_edge(CursorPosition::new(0, 500));
        assert!(result.is_none());
    }
}
