//! Tottho - Modern Offline Database Management Tool
//! 
//! A powerful, offline-first database management application built with Rust and GPUI,
//! following Zed's architectural patterns for performance and extensibility.

use std::sync::Arc;
use anyhow::Result;
use gpui::App;

// Import core modules
mod core;

use core::{TotthoCore, TotthoApp, init_logging};

fn main() -> Result<()> {
    // Initialize logging system
    init_logging()?;
    
    tracing::info!("Starting Tottho v{}", core::VERSION);
    
    // Create the core application instance
    let core = Arc::new(TotthoCore::new());
    let _app = TotthoApp::new(core.clone());
    
    // Initialize the core systems
    tokio::runtime::Runtime::new()?.block_on(async {
        if let Err(e) = core.initialize().await {
            tracing::error!("Failed to initialize application: {}", e);
            return Err(e.into());
        }
        
        tracing::info!("Tottho core systems initialized successfully");
        
        // For now, just run a simple loop to keep the application alive
        // This will be replaced with proper GPUI integration in later tasks
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    })
}
