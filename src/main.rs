//! Tottho - Modern Offline Database Management Tool
//! 
//! A powerful, offline-first database management application built with Rust and GPUI,
//! following Zed's architectural patterns for performance and extensibility.

use anyhow::Result;
use gpui::{App, Application};

// Import core modules
mod core;

use core::TotthoApp;

fn main() -> Result<()> {
    // Create and run the GPUI application following Zed's pattern
    Application::new().run(|cx: &mut App| {
        tracing::info!("Starting Tottho v{}", core::VERSION);
        
        // For now, create a simple TotthoApp instance
        // TODO: Implement proper async startup sequence when we understand GPUI patterns better
        let _app = TotthoApp::new(cx);
        tracing::info!("Tottho application created successfully");
    });
    
    Ok(())
}
