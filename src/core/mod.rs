//! Core module for Tottho application
//! 
//! This module provides the central coordination system for the Tottho database management tool,
//! following Zed's architectural patterns with gpui integration.

pub mod app;
pub mod event_bus;
pub mod module_registry;
pub mod state;
pub mod error;
pub mod config;

// Re-export core types for public API
pub use app::{TotthoCore, TotthoApp, AppPaths};
pub use event_bus::{EventBus, Event, EventHandler};
pub use module_registry::{Module, ModuleRegistry, ModuleInfo, ModuleState};
pub use state::{ApplicationState, WorkspaceLayout, UserPreferences, SessionData};
pub use error::{CoreError, Result};
pub use config::{TotthoConfig, StartupConfig, LoggingConfig, WorkspaceConfig};

/// Core module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the core logging system
pub fn init_logging() -> anyhow::Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tottho=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    Ok(())
}