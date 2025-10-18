//! Core application structure and initialization
//! 
//! This module defines the main TotthoCore struct and TotthoApp that coordinate
//! all other modules and manage the application lifecycle.

use std::sync::Arc;
use std::time::Instant;
use std::path::PathBuf;
use tokio::sync::RwLock;
use gpui::{App, AppContext, Context, Entity};
use anyhow::Result;

use crate::core::{
    EventBus, ModuleRegistry, ApplicationState, CoreError,
    TotthoConfig
};

/// Application paths for configuration, data, cache, and logs
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub logs_dir: PathBuf,
}

impl AppPaths {
    /// Create new AppPaths with platform-appropriate directories
    pub fn new() -> Result<Self, CoreError> {
        let app_name = "tottho";
        
        // Use platform-appropriate directories
        let config_dir = dirs::config_dir()
            .ok_or_else(|| CoreError::InitializationFailed { 
                reason: "Could not determine config directory".to_string() 
            })?
            .join(app_name);
        
        let data_dir = dirs::data_dir()
            .ok_or_else(|| CoreError::InitializationFailed { 
                reason: "Could not determine data directory".to_string() 
            })?
            .join(app_name);
        
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| CoreError::InitializationFailed { 
                reason: "Could not determine cache directory".to_string() 
            })?
            .join(app_name);
        
        let logs_dir = data_dir.join("logs");
        
        Ok(Self {
            config_dir,
            data_dir,
            cache_dir,
            logs_dir,
        })
    }
}

/// The central application coordinator that manages all core systems
pub struct TotthoCore {
    /// Event bus for inter-module communication
    pub event_bus: Arc<EventBus>,
    /// Registry for managing application modules
    pub module_registry: Arc<RwLock<ModuleRegistry>>,
    /// Application state management
    pub app_state: Arc<RwLock<ApplicationState>>,
    /// Configuration management
    pub config: Arc<RwLock<TotthoConfig>>,
    /// Initialization timestamp for performance tracking
    pub init_start: Instant,
}

impl TotthoCore {
    /// Create a new TotthoCore instance
    pub fn new() -> Self {
        let init_start = Instant::now();
        tracing::info!("Creating new TotthoCore instance");
        
        Self {
            event_bus: Arc::new(EventBus::new()),
            module_registry: Arc::new(RwLock::new(ModuleRegistry::new())),
            app_state: Arc::new(RwLock::new(ApplicationState::default())),
            config: Arc::new(RwLock::new(TotthoConfig::default())),
            init_start,
        }
    }

    /// Create a new TotthoCore instance with GPUI integration
    pub fn new_with_gpui(cx: &mut App) -> Entity<Self> {
        let init_start = Instant::now();
        tracing::info!("Creating new TotthoCore instance with GPUI integration");
        
        cx.new(|_cx| Self {
            event_bus: Arc::new(EventBus::new()),
            module_registry: Arc::new(RwLock::new(ModuleRegistry::new())),
            app_state: Arc::new(RwLock::new(ApplicationState::default())),
            config: Arc::new(RwLock::new(TotthoConfig::default())),
            init_start,
        })
    }

    /// Initialize the core application systems
    pub async fn initialize(&self) -> Result<(), CoreError> {
        let start_time = Instant::now();
        tracing::info!("Initializing Tottho core systems");
        
        // Load configuration first
        self.load_configuration().await?;
        
        // Initialize event bus
        self.event_bus.initialize().await?;
        
        // Load application state
        self.load_application_state().await?;
        
        // Initialize module registry
        self.initialize_module_registry().await?;
        
        let init_duration = start_time.elapsed();
        tracing::info!(
            "Tottho core systems initialized successfully in {:?}",
            init_duration
        );
        
        // Check if we met the 200ms target
        if init_duration.as_millis() > 200 {
            tracing::warn!(
                "Initialization took {:?}, exceeding 200ms target",
                init_duration
            );
        }
        
        Ok(())
    }

    /// Initialize the core application systems with GPUI context
    pub async fn initialize_with_context(&self, cx: &mut Context<'_, Self>) -> Result<(), CoreError> {
        let start_time = Instant::now();
        tracing::info!("Initializing Tottho core systems with GPUI context");
        
        // Load configuration first
        self.load_configuration().await?;
        
        // Initialize event bus
        self.event_bus.initialize().await?;
        
        // Load application state
        self.load_application_state().await?;
        
        // Initialize module registry
        self.initialize_module_registry().await?;
        
        // Notify GPUI that the model has changed
        cx.notify();
        
        let init_duration = start_time.elapsed();
        tracing::info!(
            "Tottho core systems initialized successfully with GPUI context in {:?}",
            init_duration
        );
        
        // Check if we met the 200ms target
        if init_duration.as_millis() > 200 {
            tracing::warn!(
                "Initialization took {:?}, exceeding 200ms target",
                init_duration
            );
        }
        
        Ok(())
    }

    /// Register a module with the core system
    pub async fn register_module<T: crate::core::Module + 'static>(
        &self,
        module: T,
    ) -> Result<(), CoreError> {
        let mut registry = self.module_registry.write().await;
        registry.register(Box::new(module))?;
        Ok(())
    }

    /// Shutdown the core application systems
    pub async fn shutdown(&self) -> Result<(), CoreError> {
        tracing::info!("Shutting down Tottho core systems");
        
        // Save application state
        self.save_application_state().await?;
        
        // Shutdown modules
        let registry = self.module_registry.read().await;
        registry.shutdown_all().await?;
        
        // Shutdown event bus
        self.event_bus.shutdown().await?;
        
        tracing::info!("Tottho core systems shutdown complete");
        Ok(())
    }

    async fn load_configuration(&self) -> Result<(), CoreError> {
        // TODO: Implement configuration loading
        tracing::debug!("Configuration loading placeholder");
        Ok(())
    }

    async fn load_application_state(&self) -> Result<(), CoreError> {
        // TODO: Implement state loading
        tracing::debug!("Application state loading placeholder");
        Ok(())
    }

    async fn save_application_state(&self) -> Result<(), CoreError> {
        // TODO: Implement state saving
        tracing::debug!("Application state saving placeholder");
        Ok(())
    }

    async fn initialize_module_registry(&self) -> Result<(), CoreError> {
        // TODO: Implement module registry initialization
        tracing::debug!("Module registry initialization placeholder");
        Ok(())
    }
}

impl Default for TotthoCore {
    fn default() -> Self {
        Self::new()
    }
}

/// GPUI Application wrapper for Tottho following Zed's pattern
pub struct TotthoApp {
    core: Entity<TotthoCore>,
}

impl TotthoApp {
    /// Create a new TotthoApp instance with GPUI integration
    pub fn new(cx: &mut App) -> Self {
        let core = TotthoCore::new_with_gpui(cx);
        Self { core }
    }

    /// Initialize application paths similar to Zed's init_paths()
    pub fn init_paths() -> Result<AppPaths, CoreError> {
        let app_paths = AppPaths::new()?;
        
        // Ensure directories exist
        std::fs::create_dir_all(&app_paths.config_dir)
            .map_err(|e| CoreError::InitializationFailed { 
                reason: format!("Failed to create config directory: {}", e) 
            })?;
        
        std::fs::create_dir_all(&app_paths.data_dir)
            .map_err(|e| CoreError::InitializationFailed { 
                reason: format!("Failed to create data directory: {}", e) 
            })?;
        
        std::fs::create_dir_all(&app_paths.cache_dir)
            .map_err(|e| CoreError::InitializationFailed { 
                reason: format!("Failed to create cache directory: {}", e) 
            })?;
        
        std::fs::create_dir_all(&app_paths.logs_dir)
            .map_err(|e| CoreError::InitializationFailed { 
                reason: format!("Failed to create logs directory: {}", e) 
            })?;
        
        tracing::info!("Application paths initialized: {:?}", app_paths);
        Ok(app_paths)
    }

    /// Initialize logging system with file output
    pub fn init_logging(paths: &AppPaths) -> Result<(), CoreError> {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt, EnvFilter};
        use std::fs::OpenOptions;
        
        // Create log file
        let log_file = paths.logs_dir.join("tottho.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .map_err(|e| CoreError::InitializationFailed { 
                reason: format!("Failed to create log file: {}", e) 
            })?;
        
        // Set up tracing subscriber with both console and file output
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "tottho=debug,info".into());
        
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .with_writer(std::io::stdout)
                    .with_ansi(true)
            )
            .with(
                fmt::layer()
                    .with_writer(file)
                    .with_ansi(false)
            )
            .init();
        
        tracing::info!("Logging system initialized with file output to: {:?}", log_file);
        Ok(())
    }

    /// Complete startup sequence following Zed's pattern with graceful error handling
    pub async fn startup_sequence(cx: &mut App) -> Result<Self, CoreError> {
        let startup_start = Instant::now();
        tracing::info!("Starting Tottho application startup sequence");
        
        // Phase 1: Initialize paths and logging (should be very fast)
        let paths = match Self::init_paths() {
            Ok(paths) => paths,
            Err(e) => {
                eprintln!("Failed to initialize application paths: {}", e);
                return Err(e);
            }
        };
        
        if let Err(e) = Self::init_logging(&paths) {
            eprintln!("Failed to initialize logging system: {}", e);
            return Err(e);
        }
        
        // Phase 2: Create GPUI application instance with error handling
        let app = Self::new(cx);
        tracing::debug!("Created TotthoApp instance");
        
        // Phase 3: Initialize core systems with 200ms target and graceful error handling
        match app.initialize(cx).await {
            Ok(_) => {
                tracing::info!("Core systems initialized successfully");
            }
            Err(e) => {
                tracing::error!("Failed to initialize core systems: {}", e);
                // Attempt graceful cleanup before returning error
                if let Err(cleanup_err) = app.graceful_shutdown().await {
                    tracing::error!("Failed to cleanup after initialization failure: {}", cleanup_err);
                }
                return Err(e);
            }
        }
        
        let startup_duration = startup_start.elapsed();
        tracing::info!(
            "Tottho application startup sequence completed in {:?}",
            startup_duration
        );
        
        // Check if we met the 200ms target for the entire startup
        if startup_duration.as_millis() > 200 {
            tracing::warn!(
                "Startup sequence took {:?}, exceeding 200ms target",
                startup_duration
            );
        } else {
            tracing::info!(
                "Startup sequence completed within 200ms target ({:?})",
                startup_duration
            );
        }
        
        Ok(app)
    }

    /// Graceful shutdown for cleanup during initialization failures
    pub async fn graceful_shutdown(&self) -> Result<(), CoreError> {
        tracing::info!("Performing graceful shutdown");
        
        // For now, just log the shutdown attempt
        // TODO: Implement proper shutdown when we have access to the GPUI context
        tracing::warn!("Graceful shutdown is a placeholder - proper implementation requires GPUI context");
        
        tracing::info!("Graceful shutdown completed");
        Ok(())
    }

    /// Initialize the GPUI application
    pub async fn initialize(&self, cx: &App) -> Result<(), CoreError> {
        // For now, just call the regular initialization
        // TODO: Implement proper GPUI async initialization when we understand the patterns better
        tracing::info!("TotthoApp initialization started");
        
        // Get a reference to the core and initialize it
        let core_ref = &*self.core.read(cx);
        core_ref.initialize().await?;
        
        tracing::info!("TotthoApp initialization completed successfully");
        Ok(())
    }

    /// Get a reference to the core entity
    pub fn core(&self) -> &Entity<TotthoCore> {
        &self.core
    }
}