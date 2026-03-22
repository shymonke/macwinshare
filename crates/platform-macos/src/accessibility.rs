//! macOS Accessibility Permission Handling
//! 
//! Checks and requests accessibility permissions required for input capture.

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use tracing::{info, warn};

// External function declarations for accessibility API
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
}

/// Check if the application has accessibility permissions
pub fn check_accessibility() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Check accessibility with option to prompt user
pub fn check_accessibility_with_prompt(prompt: bool) -> bool {
    if prompt {
        // Create options dictionary with kAXTrustedCheckOptionPrompt = true
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let value = if prompt { CFBoolean::true_value() } else { CFBoolean::false_value() };
        
        // from_CFType_pairs takes a slice of tuples (key, value)
        let pairs = [(key, value)];
        let options = CFDictionary::from_CFType_pairs(&pairs);
        
        unsafe {
            AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef() as *const _)
        }
    } else {
        check_accessibility()
    }
}

/// Request accessibility permissions (opens System Preferences)
pub fn request_accessibility() {
    if !check_accessibility() {
        warn!("Accessibility permission not granted. Opening System Settings...");
        
        // This will trigger the system prompt for accessibility access
        check_accessibility_with_prompt(true);
        
        // Also try to open System Settings directly
        let _ = std::process::Command::new("open")
            .args([
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
            ])
            .spawn();
        
        info!("Please grant accessibility access in System Settings > Privacy & Security > Accessibility");
    } else {
        info!("Accessibility permission already granted");
    }
}

/// Get a user-friendly message about accessibility requirements
pub fn get_accessibility_message() -> &'static str {
    if check_accessibility() {
        "Accessibility access is enabled."
    } else {
        "MacWinShare requires accessibility access to capture and inject input events.\n\n\
         Please go to System Settings > Privacy & Security > Accessibility\n\
         and enable access for MacWinShare."
    }
}
