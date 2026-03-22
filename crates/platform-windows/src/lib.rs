//! Windows Platform Implementation
//! 
//! Provides input capture and injection using Windows APIs:
//! - SetWindowsHookEx for input capture
//! - SendInput for input injection

pub mod input;
pub mod display;

pub use input::WindowsInput;
pub use display::WindowsDisplay;
