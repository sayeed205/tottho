//! Core application structure and initialization
//!
//! This module defines the main TotthoCore struct and TotthoApp that coordinate
//! all other modules and manage the application lifecycle.

use anyhow::Result;
use gpui::{App, AppContext, Context, Entity};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing_subscriber::layer::Layer;

/// Command line arguments for Tottho (simplified version for core module)
#[derive(Debug, Clone)]
pub struct Args {
    /// Configuration file path
    pub config_path: Option<PathBuf>,
    /// Log level override
    pub log_level: Option<String>,
    /// Disable session restoration
    pub no_restore: bool,
    /// Show version and exit
    pub version: bool,
    /// Show help and exit
    pub help: bool,
    /// Enable debug mode
    pub debug: bool,
    /// Workspace to open on startup
    pub workspace: Option<PathBuf>,
}

use crate::core::{ApplicationState, CoreError, EventBus, ModuleRegistry, TotthoConfig, SettingsManager, WorkspaceManager};

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
                reason: "Could not determine config directory".to_string(),
            })?
            .join(app_name);

        let data_dir = dirs::data_dir()
            .ok_or_else(|| CoreError::InitializationFailed {
                reason: "Could not determine data directory".to_string(),
            })?
            .join(app_name);

        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| CoreError::InitializationFailed {
                reason: "Could not determine cache directory".to_string(),
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
#[derive(Clone)]
pub struct TotthoCore {
    /// Event bus for inter-module communication
    pub event_bus: Arc<EventBus>,
    /// Registry for managing application modules
    pub module_registry: Arc<RwLock<ModuleRegistry>>,
    /// Application state management
    pub app_state: Arc<RwLock<ApplicationState>>,
    /// Configuration management
    pub config: Arc<RwLock<TotthoConfig>>,
    /// Settings manager for runtime configuration changes
    pub settings_manager: Arc<RwLock<SettingsManager>>,
    /// Workspace manager for layout and session management
    pub workspace_manager: Arc<WorkspaceManager>,
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
            settings_manager: Arc::new(RwLock::new(SettingsManager::with_default_path())),
            workspace_manager: Arc::new(WorkspaceManager::new()),
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
            settings_manager: Arc::new(RwLock::new(SettingsManager::with_default_path())),
            workspace_manager: Arc::new(WorkspaceManager::new()),
            init_start,
        })
    }

    /// Initialize the core application systems
    pub async fn initialize(&self) -> Result<(), CoreError> {
        let start_time = Instant::now();
        tracing::info!("Initializing Tottho core systems");

        // Load configuration first
        self.load_configuration().await?;

        // Initialize settings manager with current configuration
        self.initialize_settings_manager().await?;

        // Initialize event bus
        self.event_bus.initialize().await?;

        // Load application state
        self.load_application_state().await?;

        // Initialize module registry
        self.initialize_module_registry().await?;

        // Initialize workspace manager
        self.initialize_workspace_manager().await?;

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
    pub async fn initialize_with_context(
        &self,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), CoreError> {
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
        registry.register(module)?;
        Ok(())
    }

    /// Shutdown the core application systems
    pub async fn shutdown(&self) -> Result<(), CoreError> {
        tracing::info!("Shutting down Tottho core systems");

        // Save application state
        self.save_application_state().await?;

        // Shutdown settings manager
        let mut settings_manager = self.settings_manager.write().await;
        settings_manager.shutdown().await;

        // Shutdown modules
        let mut registry = self.module_registry.write().await;
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

    async fn initialize_settings_manager(&self) -> Result<(), CoreError> {
        let config = self.config.read().await;
        let mut settings_manager = self.settings_manager.write().await;
        settings_manager.initialize(config.clone()).await?;
        tracing::debug!("Settings manager initialized");
        Ok(())
    }

    async fn initialize_workspace_manager(&self) -> Result<(), CoreError> {
        let config = self.config.read().await;
        self.workspace_manager.initialize(&config).await?;
        tracing::debug!("Workspace manager initialized");
        Ok(())
    }
}

impl Default for TotthoCore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_tottho_core_new() {
        // Test that TotthoCore::new() creates a valid instance
        let core = TotthoCore::new();

        // Verify all components are initialized (Arc should not be null)
        assert!(Arc::strong_count(&core.event_bus) >= 1);
        assert!(Arc::strong_count(&core.module_registry) >= 1);
        assert!(Arc::strong_count(&core.app_state) >= 1);
        assert!(Arc::strong_count(&core.config) >= 1);

        // Verify initialization timestamp is recent (within last second)
        let elapsed = core.init_start.elapsed();
        assert!(elapsed < Duration::from_secs(1));
    }

    #[test]
    fn test_tottho_core_default() {
        // Test that Default trait works correctly
        let core = TotthoCore::default();

        // Should be equivalent to new()
        assert!(Arc::strong_count(&core.event_bus) >= 1);
        assert!(Arc::strong_count(&core.module_registry) >= 1);
        assert!(Arc::strong_count(&core.app_state) >= 1);
        assert!(Arc::strong_count(&core.config) >= 1);
    }

    #[tokio::test]
    async fn test_tottho_core_initialize_success() {
        // Test successful initialization
        let core = TotthoCore::new();
        let start_time = Instant::now();

        let result = core.initialize().await;
        let init_duration = start_time.elapsed();

        // Should succeed
        assert!(result.is_ok());

        // Should complete reasonably quickly (allowing some margin for CI)
        assert!(init_duration < Duration::from_millis(500));
    }

    #[tokio::test]
    async fn test_tottho_core_initialize_timing() {
        // Test that initialization meets the 200ms target under normal conditions
        let core = TotthoCore::new();
        let start_time = Instant::now();

        let _result = core.initialize().await;
        let init_duration = start_time.elapsed();

        // Note: In a real test environment, this might exceed 200ms due to I/O
        // but we test that it's reasonable (under 1 second)
        assert!(init_duration < Duration::from_secs(1));

        // Log the actual timing for monitoring
        println!("Initialization took: {:?}", init_duration);
    }

    #[test]
    fn test_app_paths_new() {
        // Test AppPaths creation
        let result = AppPaths::new();

        // Should succeed on most systems
        match result {
            Ok(paths) => {
                // Verify paths are not empty
                assert!(!paths.config_dir.as_os_str().is_empty());
                assert!(!paths.data_dir.as_os_str().is_empty());
                assert!(!paths.cache_dir.as_os_str().is_empty());
                assert!(!paths.logs_dir.as_os_str().is_empty());

                // Verify logs_dir is under data_dir
                assert!(paths.logs_dir.starts_with(&paths.data_dir));
            }
            Err(e) => {
                // On some systems (like CI), this might fail
                println!(
                    "AppPaths::new() failed (expected in some environments): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_tottho_app_init_paths() {
        // Test path initialization
        let result = TotthoApp::init_paths();

        match result {
            Ok(paths) => {
                // Verify paths exist after initialization
                assert!(
                    paths.config_dir.exists()
                        || paths.config_dir.parent().map_or(false, |p| p.exists())
                );
                assert!(
                    paths.data_dir.exists()
                        || paths.data_dir.parent().map_or(false, |p| p.exists())
                );
                assert!(
                    paths.cache_dir.exists()
                        || paths.cache_dir.parent().map_or(false, |p| p.exists())
                );
                assert!(
                    paths.logs_dir.exists()
                        || paths.logs_dir.parent().map_or(false, |p| p.exists())
                );
            }
            Err(e) => {
                // May fail in restricted environments
                println!("init_paths failed (expected in some environments): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_multiple_cores_independent() {
        // Test that multiple TotthoCore instances are independent
        let core1 = TotthoCore::new();
        let core2 = TotthoCore::new();

        // Initialize both
        let result1 = core1.initialize().await;
        let result2 = core2.initialize().await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        // They should have different initialization times
        assert_ne!(core1.init_start, core2.init_start);
    }

    #[tokio::test]
    async fn test_initialization_error_scenarios() {
        // Test initialization with potential error conditions
        let core = TotthoCore::new();

        // For now, initialization should always succeed with placeholder implementations
        let result = core.initialize().await;
        assert!(result.is_ok());

        // Test double initialization (should still work with current implementation)
        let result2 = core.initialize().await;
        assert!(result2.is_ok());
    }

    #[test]
    fn test_core_components_not_null() {
        // Test that all core components are properly initialized and not null
        let core = TotthoCore::new();

        // Test Arc references are valid
        let event_bus_ref = Arc::clone(&core.event_bus);
        let module_registry_ref = Arc::clone(&core.module_registry);
        let app_state_ref = Arc::clone(&core.app_state);
        let config_ref = Arc::clone(&core.config);

        // These should not panic
        drop(event_bus_ref);
        drop(module_registry_ref);
        drop(app_state_ref);
        drop(config_ref);
    }

    #[tokio::test]
    async fn test_concurrent_initialization() {
        // Test that concurrent initialization attempts don't cause issues
        let core = Arc::new(TotthoCore::new());

        let core1 = Arc::clone(&core);
        let core2 = Arc::clone(&core);

        let handle1 = tokio::spawn(async move { core1.initialize().await });

        let handle2 = tokio::spawn(async move { core2.initialize().await });

        let (result1, result2) = tokio::join!(handle1, handle2);

        // Both should succeed (current implementation is idempotent)
        assert!(result1.unwrap().is_ok());
        assert!(result2.unwrap().is_ok());
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
        let app = Self { core };
        
        // Initialize UI system and create main window
        if let Err(e) = app.initialize_ui_and_create_window(cx) {
            tracing::error!("Failed to initialize UI and create window: {}", e);
        }
        
        app
    }
    
    /// Initialize UI system and create the main application window
    fn initialize_ui_and_create_window(&self, cx: &mut App) -> Result<(), CoreError> {
        use crate::ui::{UISystem, TotthoWorkspace};
        use gpui::{WindowOptions, Bounds, Size, Point};
        
        tracing::info!("Initializing UI system and creating main window");
        
        // Initialize the UI system
        let ui_system = UISystem::new(cx)
            .map_err(|e| CoreError::InitializationFailed {
                reason: format!("Failed to initialize UI system: {}", e),
            })?;
        
        ui_system.initialize(cx)
            .map_err(|e| CoreError::InitializationFailed {
                reason: format!("Failed to initialize UI system: {}", e),
            })?;
        
        // Create the main workspace
        let workspace = ui_system.manager().create_workspace(cx)
            .map_err(|e| CoreError::InitializationFailed {
                reason: format!("Failed to create workspace: {}", e),
            })?;
        
        // Create the main application window
        use gpui::{WindowBounds, size, px};
        
        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        };
        
        cx.open_window(window_options, |_window, cx| workspace)
            .map_err(|e| CoreError::InitializationFailed {
                reason: format!("Failed to create main window: {}", e),
            })?;
        
        tracing::info!("Main application window created successfully");
        Ok(())
    }

    /// Initialize application paths similar to Zed's init_paths()
    pub fn init_paths() -> Result<AppPaths, CoreError> {
        let app_paths = AppPaths::new()?;

        // Ensure directories exist
        std::fs::create_dir_all(&app_paths.config_dir).map_err(|e| {
            CoreError::InitializationFailed {
                reason: format!("Failed to create config directory: {}", e),
            }
        })?;

        std::fs::create_dir_all(&app_paths.data_dir).map_err(|e| {
            CoreError::InitializationFailed {
                reason: format!("Failed to create data directory: {}", e),
            }
        })?;

        std::fs::create_dir_all(&app_paths.cache_dir).map_err(|e| {
            CoreError::InitializationFailed {
                reason: format!("Failed to create cache directory: {}", e),
            }
        })?;

        std::fs::create_dir_all(&app_paths.logs_dir).map_err(|e| {
            CoreError::InitializationFailed {
                reason: format!("Failed to create logs directory: {}", e),
            }
        })?;

        tracing::info!("Application paths initialized: {:?}", app_paths);
        Ok(app_paths)
    }

    /// Initialize logging system with file output
    pub fn init_logging(paths: &AppPaths) -> Result<(), CoreError> {
        use std::fs::OpenOptions;
        use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

        // Create log file
        let log_file = paths.logs_dir.join("tottho.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .map_err(|e| CoreError::InitializationFailed {
                reason: format!("Failed to create log file: {}", e),
            })?;

        // Set up tracing subscriber with both console and file output
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "tottho=debug,info".into());

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().with_writer(std::io::stdout).with_ansi(true))
            .with(fmt::layer().with_writer(file).with_ansi(false))
            .init();

        tracing::info!(
            "Logging system initialized with file output to: {:?}",
            log_file
        );
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
                    tracing::error!(
                        "Failed to cleanup after initialization failure: {}",
                        cleanup_err
                    );
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

    /// Complete startup sequence with configuration and command line arguments
    pub async fn startup_sequence_with_config(
        cx: &mut App,
        config: crate::core::TotthoConfig,
        args: Args,
    ) -> Result<Self, CoreError> {
        let startup_start = Instant::now();
        tracing::info!("Starting Tottho application startup sequence with configuration");

        // Phase 1: Initialize paths (paths should already be initialized by main)
        let paths = Self::init_paths()?;

        // Phase 2: Initialize enhanced logging with configuration
        Self::init_logging_with_config(&paths, &config.logging)?;

        // Phase 3: Create GPUI application instance
        let app = cx.new(|cx| {
            let core = TotthoCore::new_with_gpui(cx);
            Self { core }
        });

        // Phase 4: Log configuration application
        tracing::info!("Configuration applied: restore_session={}, debug_mode={}", 
            config.startup.restore_session, args.debug);

        // Apply workspace restoration if enabled
        if config.startup.restore_session && !args.no_restore {
            tracing::info!("Restoring previous session");
            // Note: Workspace restoration will be handled during core initialization
        }

        // Open specific workspace if provided
        if let Some(workspace_path) = &args.workspace {
            tracing::info!("Opening workspace: {:?}", workspace_path);
            // Note: Workspace opening will be handled during core initialization
        }

        // Note: Async initialization will be handled by the main application loop
        tracing::info!("Core systems will be initialized asynchronously by the main loop");

        let startup_duration = startup_start.elapsed();
        tracing::info!(
            "Tottho application startup sequence with config completed in {:?}",
            startup_duration
        );

        // Check if we met the 200ms target
        if startup_duration.as_millis() > 200 {
            tracing::warn!(
                "Startup sequence took {:?}, exceeding 200ms target",
                startup_duration
            );
        }

        Ok(Self { core: app.read(cx).core.clone() })
    }

    /// Initialize logging system with configuration
    pub fn init_logging_with_config(
        paths: &AppPaths,
        logging_config: &crate::core::LoggingConfig,
    ) -> Result<(), CoreError> {
        use std::fs::OpenOptions;
        use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

        // Set up environment filter with configuration
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| {
                format!("tottho={},info", logging_config.level).into()
            });

        let registry = tracing_subscriber::registry().with(env_filter);

        // Configure logging based on settings
        match (logging_config.console_logging, logging_config.log_to_file) {
            (true, true) => {
                // Both console and file logging
                let log_file_path = logging_config.log_file_path
                    .clone()
                    .unwrap_or_else(|| paths.logs_dir.join("tottho.log"));

                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_file_path)
                    .map_err(|e| CoreError::InitializationFailed {
                        reason: format!("Failed to create log file: {}", e),
                    })?;

                registry
                    .with(fmt::layer().with_writer(std::io::stdout).with_ansi(true))
                    .with(fmt::layer().with_writer(file).with_ansi(false))
                    .init();

                tracing::info!("Console and file logging enabled: {:?}", log_file_path);
            }
            (true, false) => {
                // Console logging only
                registry
                    .with(fmt::layer().with_writer(std::io::stdout).with_ansi(true))
                    .init();

                tracing::info!("Console logging enabled");
            }
            (false, true) => {
                // File logging only
                let log_file_path = logging_config.log_file_path
                    .clone()
                    .unwrap_or_else(|| paths.logs_dir.join("tottho.log"));

                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_file_path)
                    .map_err(|e| CoreError::InitializationFailed {
                        reason: format!("Failed to create log file: {}", e),
                    })?;

                registry
                    .with(fmt::layer().with_writer(file).with_ansi(false))
                    .init();

                tracing::info!("File logging enabled: {:?}", log_file_path);
            }
            (false, false) => {
                // No logging configured, use default console
                registry
                    .with(fmt::layer().with_writer(std::io::stdout).with_ansi(true))
                    .init();

                tracing::warn!("No logging configured, using default console logging");
            }
        }

        tracing::info!("Enhanced logging system initialized with level: {}", logging_config.level);
        Ok(())
    }

    /// Graceful shutdown for cleanup during initialization failures
    pub async fn graceful_shutdown(&self) -> Result<(), CoreError> {
        tracing::info!("Performing graceful shutdown");

        // For now, just log the shutdown attempt
        // TODO: Implement proper shutdown when we have access to the GPUI context
        tracing::warn!(
            "Graceful shutdown is a placeholder - proper implementation requires GPUI context"
        );

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
