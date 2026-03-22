//! macOS Platform Implementation
//! 
//! Provides input capture and injection using macOS APIs:
//! - CoreGraphics for event tap and posting
//! - IOKit for HID access

pub mod input;
pub mod display;
pub mod accessibility;

pub use input::MacOSInput;
pub use display::MacOSDisplay;
pub use accessibility::check_accessibility;
