//! Configuration management for the Tottho application
//! 
//! Handles loading, validation, and persistence of application configuration.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;

use crate::core::{CoreError, Result};

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotthoConfig {
    /// Startup configuration
    pub startup: StartupConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Module-specific configurations
    pub modules: HashMap<String, ModuleConfig>,
    /// Workspace configuration
    pub workspace: WorkspaceConfig,
}

/// Startup configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupConfig {
    /// Whether to restore the previous session on startup
    pub restore_session: bool,
    /// Startup timeout in milliseconds
    pub startup_timeout_ms: u64,
    /// Enable crash recovery
    pub crash_recovery: bool,
    /// Show splash screen during startup
    pub show_splash: bool,
    /// Automatically check for updates
    pub auto_update_check: bool,
}

impl Default for StartupConfig {
    fn default() -> Self {
        Self {
            restore_session: true,
            startup_timeout_ms: 5000,
            crash_recovery: true,
            show_splash: true,
            auto_update_check: true,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Log to file
    pub log_to_file: bool,
    /// Log file path
    pub log_file_path: Option<PathBuf>,
    /// Maximum log file size in MB
    pub max_file_size_mb: u64,
    /// Number of log files to keep
    pub max_files: u32,
    /// Enable console logging
    pub console_logging: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_to_file: true,
            log_file_path: None, // Will be set to default location
            max_file_size_mb: 10,
            max_files: 5,
            console_logging: true,
        }
    }
}

/// Module-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// Whether the module is enabled
    pub enabled: bool,
    /// Module-specific settings
    pub settings: HashMap<String, serde_json::Value>,
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            settings: HashMap::new(),
        }
    }
}

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Default workspace layout
    pub default_layout: String,
    /// Auto-save workspace state
    pub auto_save_state: bool,
    /// Auto-save interval in seconds
    pub auto_save_interval_sec: u64,
    /// Maximum number of recent workspaces to remember
    pub max_recent_workspaces: usize,
    /// Default window size
    pub default_window_size: WindowSize,
}

/// Window size configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            default_layout: "default".to_string(),
            auto_save_state: true,
            auto_save_interval_sec: 30,
            max_recent_workspaces: 10,
            default_window_size: WindowSize {
                width: 1200,
                height: 800,
            },
        }
    }
}

impl TotthoConfig {
    /// Create a new configuration with defaults
    pub fn new() -> Self {
        Self {
            startup: StartupConfig::default(),
            logging: LoggingConfig::default(),
            modules: HashMap::new(),
            workspace: WorkspaceConfig::default(),
        }
    }

    /// Load configuration from file
    pub async fn load_from_file(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            tracing::info!("Configuration file not found, using defaults: {:?}", path);
            return Ok(Self::new());
        }

        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            CoreError::ConfigurationError {
                reason: format!("Failed to read configuration file: {}", e),
            }
        })?;

        let config: TotthoConfig = toml::from_str(&content).map_err(|e| {
            CoreError::InvalidConfigurationFormat {
                reason: format!("Invalid TOML format: {}", e),
            }
        })?;

        config.validate()?;
        tracing::info!("Configuration loaded from: {:?}", path);
        Ok(config)
    }

    /// Save configuration to file
    pub async fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                CoreError::ConfigurationError {
                    reason: format!("Failed to create config directory: {}", e),
                }
            })?;
        }

        let content = toml::to_string_pretty(self).map_err(|e| {
            CoreError::ConfigurationError {
                reason: format!("Failed to serialize configuration: {}", e),
            }
        })?;

        tokio::fs::write(path, content).await.map_err(|e| {
            CoreError::ConfigurationError {
                reason: format!("Failed to write configuration file: {}", e),
            }
        })?;

        tracing::info!("Configuration saved to: {:?}", path);
        Ok(())
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate startup timeout
        if self.startup.startup_timeout_ms == 0 {
            return Err(CoreError::InvalidConfigurationFormat {
                reason: "Startup timeout must be greater than 0".to_string(),
            });
        }

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(CoreError::InvalidConfigurationFormat {
                reason: format!("Invalid log level: {}. Must be one of: {:?}", 
                    self.logging.level, valid_levels),
            });
        }

        // Validate log file size
        if self.logging.max_file_size_mb == 0 {
            return Err(CoreError::InvalidConfigurationFormat {
                reason: "Log file size must be greater than 0".to_string(),
            });
        }

        // Validate window size
        if self.workspace.default_window_size.width < 400 || 
           self.workspace.default_window_size.height < 300 {
            return Err(CoreError::InvalidConfigurationFormat {
                reason: "Window size must be at least 400x300".to_string(),
            });
        }

        Ok(())
    }

    /// Get module configuration
    pub fn get_module_config(&self, module_name: &str) -> ModuleConfig {
        self.modules.get(module_name).cloned().unwrap_or_default()
    }

    /// Set module configuration
    pub fn set_module_config(&mut self, module_name: String, config: ModuleConfig) {
        self.modules.insert(module_name, config);
    }

    /// Check if a module is enabled
    pub fn is_module_enabled(&self, module_name: &str) -> bool {
        self.modules.get(module_name)
            .map(|config| config.enabled)
            .unwrap_or(true) // Default to enabled if not specified
    }

    /// Get default configuration file path
    pub fn default_config_path() -> PathBuf {
        // This would typically be in the user's config directory
        // For now, use a simple path relative to the executable
        PathBuf::from("tottho.toml")
    }

    /// Merge with another configuration (other takes precedence)
    pub fn merge(&mut self, other: TotthoConfig) {
        // Merge startup config
        if other.startup.restore_session != self.startup.restore_session {
            self.startup.restore_session = other.startup.restore_session;
        }
        if other.startup.startup_timeout_ms != 5000 { // Not default
            self.startup.startup_timeout_ms = other.startup.startup_timeout_ms;
        }
        
        // Merge logging config
        if other.logging.level != "info" { // Not default
            self.logging.level = other.logging.level;
        }
        
        // Merge module configs
        for (name, config) in other.modules {
            self.modules.insert(name, config);
        }
        
        // Merge workspace config
        if other.workspace.default_layout != "default" {
            self.workspace.default_layout = other.workspace.default_layout;
        }
    }
}

impl Default for TotthoConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Settings manager for runtime configuration changes and persistence
pub struct SettingsManager {
    /// Current configuration
    config: Arc<RwLock<TotthoConfig>>,
    /// Configuration file path
    config_path: PathBuf,
    /// Settings change subscribers
    subscribers: Arc<RwLock<Vec<SettingsChangeCallback>>>,
    /// Auto-save task handle
    auto_save_handle: Option<tokio::task::JoinHandle<()>>,
    /// File watcher for configuration changes
    _file_watcher: Option<tokio::task::JoinHandle<()>>,
}

impl std::fmt::Debug for SettingsManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettingsManager")
            .field("config_path", &self.config_path)
            .field("auto_save_handle", &self.auto_save_handle.is_some())
            .field("file_watcher", &self._file_watcher.is_some())
            .field("subscribers_count", &self.subscribers.try_read().map(|s| s.len()).unwrap_or(0))
            .finish()
    }
}

/// Callback for settings changes
pub type SettingsChangeCallback = Arc<dyn Fn(&TotthoConfig, &TotthoConfig) + Send + Sync>;

/// Settings change event
#[derive(Debug, Clone)]
pub struct SettingsChangeEvent {
    /// Previous configuration
    pub previous: TotthoConfig,
    /// New configuration
    pub current: TotthoConfig,
    /// Timestamp of the change
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Source of the change (user, file, system)
    pub source: SettingsChangeSource,
}

/// Source of settings change
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsChangeSource {
    /// User-initiated change through UI
    User,
    /// File system change (external edit)
    File,
    /// System-initiated change
    System,
    /// Auto-reload from file
    AutoReload,
}

impl SettingsManager {
    /// Create a new settings manager
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config: Arc::new(RwLock::new(TotthoConfig::default())),
            config_path,
            subscribers: Arc::new(RwLock::new(Vec::new())),
            auto_save_handle: None,
            _file_watcher: None,
        }
    }

    /// Initialize the settings manager with configuration
    pub async fn initialize(&mut self, initial_config: TotthoConfig) -> Result<()> {
        // Set initial configuration
        {
            let mut config = self.config.write().await;
            *config = initial_config.clone();
        }

        // Start auto-save if enabled
        if initial_config.workspace.auto_save_state {
            self.start_auto_save(initial_config.workspace.auto_save_interval_sec).await;
        }

        // Start file watcher for auto-reload
        self.start_file_watcher().await?;

        tracing::info!("Settings manager initialized with config path: {:?}", self.config_path);
        Ok(())
    }

    /// Get current configuration
    pub async fn get_config(&self) -> TotthoConfig {
        self.config.read().await.clone()
    }

    /// Update configuration
    pub async fn update_config<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut TotthoConfig) -> Result<()>,
    {
        let previous_config = {
            let config = self.config.read().await;
            config.clone()
        };

        let new_config = {
            let mut config = self.config.write().await;
            let mut new_config = config.clone();
            updater(&mut new_config)?;
            new_config.validate()?;
            *config = new_config.clone();
            new_config
        };

        // Notify subscribers of the change
        self.notify_subscribers(&previous_config, &new_config, SettingsChangeSource::User).await;

        tracing::info!("Configuration updated");
        Ok(())
    }

    /// Save configuration to file
    pub async fn save_config(&self) -> Result<()> {
        let config = self.config.read().await;
        config.save_to_file(&self.config_path).await?;
        tracing::debug!("Configuration saved to file: {:?}", self.config_path);
        Ok(())
    }

    /// Reload configuration from file
    pub async fn reload_config(&self) -> Result<()> {
        let previous_config = {
            let config = self.config.read().await;
            config.clone()
        };

        let new_config = TotthoConfig::load_from_file(&self.config_path).await?;
        
        {
            let mut config = self.config.write().await;
            *config = new_config.clone();
        }

        // Notify subscribers of the change
        self.notify_subscribers(&previous_config, &new_config, SettingsChangeSource::AutoReload).await;

        tracing::info!("Configuration reloaded from file");
        Ok(())
    }

    /// Subscribe to configuration changes
    pub async fn subscribe_to_changes(&self, callback: SettingsChangeCallback) {
        let mut subscribers = self.subscribers.write().await;
        subscribers.push(callback);
    }

    /// Get a specific setting value
    pub async fn get_setting<T>(&self, getter: impl Fn(&TotthoConfig) -> T) -> T {
        let config = self.config.read().await;
        getter(&config)
    }

    /// Update a specific setting
    pub async fn update_setting<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut TotthoConfig) -> Result<()>,
    {
        self.update_config(updater).await
    }

    /// Get module configuration
    pub async fn get_module_config(&self, module_name: &str) -> ModuleConfig {
        let config = self.config.read().await;
        config.get_module_config(module_name)
    }

    /// Update module configuration
    pub async fn update_module_config(&self, module_name: String, module_config: ModuleConfig) -> Result<()> {
        self.update_config(|config| {
            config.set_module_config(module_name, module_config);
            Ok(())
        }).await
    }

    /// Check if a module is enabled
    pub async fn is_module_enabled(&self, module_name: &str) -> bool {
        let config = self.config.read().await;
        config.is_module_enabled(module_name)
    }

    /// Enable or disable a module
    pub async fn set_module_enabled(&self, module_name: String, enabled: bool) -> Result<()> {
        self.update_config(|config| {
            let mut module_config = config.get_module_config(&module_name);
            module_config.enabled = enabled;
            config.set_module_config(module_name, module_config);
            Ok(())
        }).await
    }

    /// Start auto-save functionality
    async fn start_auto_save(&mut self, interval_sec: u64) {
        let config = self.config.clone();
        let config_path = self.config_path.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_sec));
            
            loop {
                interval.tick().await;
                
                // Save configuration
                let current_config = config.read().await;
                if let Err(e) = current_config.save_to_file(&config_path).await {
                    tracing::warn!("Auto-save failed: {}", e);
                }
            }
        });
        
        self.auto_save_handle = Some(handle);
        tracing::info!("Auto-save started with interval: {}s", interval_sec);
    }

    /// Start file watcher for configuration changes
    async fn start_file_watcher(&mut self) -> Result<()> {
        let config = self.config.clone();
        let config_path = self.config_path.clone();
        let subscribers = self.subscribers.clone();
        
        let handle = tokio::spawn(async move {
            // Simple polling-based file watcher
            // In a production implementation, you might use a proper file watcher like notify
            let mut last_modified = std::fs::metadata(&config_path)
                .ok()
                .and_then(|m| m.modified().ok());
            
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            
            loop {
                interval.tick().await;
                
                if let Ok(metadata) = std::fs::metadata(&config_path) {
                    if let Ok(modified) = metadata.modified() {
                        if last_modified.map_or(true, |last| modified > last) {
                            last_modified = Some(modified);
                            
                            // File was modified, reload configuration
                            match TotthoConfig::load_from_file(&config_path).await {
                                Ok(new_config) => {
                                    let previous_config = {
                                        let current = config.read().await;
                                        current.clone()
                                    };
                                    
                                    {
                                        let mut current = config.write().await;
                                        *current = new_config.clone();
                                    }
                                    
                                    // Notify subscribers
                                    let subscribers = subscribers.read().await;
                                    for callback in subscribers.iter() {
                                        callback(&previous_config, &new_config);
                                    }
                                    
                                    tracing::info!("Configuration auto-reloaded from file");
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to reload configuration: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        });
        
        self._file_watcher = Some(handle);
        tracing::info!("File watcher started for config path: {:?}", self.config_path);
        Ok(())
    }

    /// Notify all subscribers of configuration changes
    async fn notify_subscribers(
        &self,
        previous: &TotthoConfig,
        current: &TotthoConfig,
        source: SettingsChangeSource,
    ) {
        let subscribers = self.subscribers.read().await;
        
        for callback in subscribers.iter() {
            callback(previous, current);
        }
        
        tracing::debug!("Notified {} subscribers of configuration change from {:?}", 
            subscribers.len(), source);
    }

    /// Shutdown the settings manager
    pub async fn shutdown(&mut self) {
        // Save current configuration before shutdown
        if let Err(e) = self.save_config().await {
            tracing::warn!("Failed to save configuration during shutdown: {}", e);
        }

        // Cancel auto-save task
        if let Some(handle) = self.auto_save_handle.take() {
            handle.abort();
        }

        // Cancel file watcher task
        if let Some(handle) = self._file_watcher.take() {
            handle.abort();
        }

        tracing::info!("Settings manager shutdown complete");
    }

    /// Reset configuration to defaults
    pub async fn reset_to_defaults(&self) -> Result<()> {
        let previous_config = {
            let config = self.config.read().await;
            config.clone()
        };

        let default_config = TotthoConfig::default();
        
        {
            let mut config = self.config.write().await;
            *config = default_config.clone();
        }

        // Save the default configuration
        self.save_config().await?;

        // Notify subscribers
        self.notify_subscribers(&previous_config, &default_config, SettingsChangeSource::System).await;

        tracing::info!("Configuration reset to defaults");
        Ok(())
    }

    /// Validate current configuration
    pub async fn validate_config(&self) -> Result<()> {
        let config = self.config.read().await;
        config.validate()
    }

    /// Get configuration file path
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// Create a settings manager with default path
    pub fn with_default_path() -> Self {
        Self::new(TotthoConfig::default_config_path())
    }
}

impl Drop for SettingsManager {
    fn drop(&mut self) {
        // Cancel tasks if they're still running
        if let Some(handle) = self.auto_save_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self._file_watcher.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_settings_manager_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let mut settings_manager = SettingsManager::new(config_path.clone());
        let config = TotthoConfig::default();
        
        settings_manager.initialize(config.clone()).await.unwrap();
        
        let retrieved_config = settings_manager.get_config().await;
        assert_eq!(retrieved_config.startup.restore_session, config.startup.restore_session);
        assert_eq!(retrieved_config.logging.level, config.logging.level);
        
        settings_manager.shutdown().await;
    }

    #[tokio::test]
    async fn test_settings_manager_config_updates() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let mut settings_manager = SettingsManager::new(config_path.clone());
        let config = TotthoConfig::default();
        
        settings_manager.initialize(config).await.unwrap();
        
        // Update configuration
        settings_manager.update_config(|config| {
            config.startup.restore_session = false;
            config.logging.level = "debug".to_string();
            Ok(())
        }).await.unwrap();
        
        let updated_config = settings_manager.get_config().await;
        assert_eq!(updated_config.startup.restore_session, false);
        assert_eq!(updated_config.logging.level, "debug");
        
        settings_manager.shutdown().await;
    }

    #[tokio::test]
    async fn test_settings_manager_module_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let mut settings_manager = SettingsManager::new(config_path.clone());
        let config = TotthoConfig::default();
        
        settings_manager.initialize(config).await.unwrap();
        
        // Test module configuration
        let module_config = ModuleConfig {
            enabled: false,
            settings: {
                let mut settings = HashMap::new();
                settings.insert("test_setting".to_string(), serde_json::Value::String("test_value".to_string()));
                settings
            },
        };
        
        settings_manager.update_module_config("test_module".to_string(), module_config.clone()).await.unwrap();
        
        let retrieved_config = settings_manager.get_module_config("test_module").await;
        assert_eq!(retrieved_config.enabled, false);
        assert_eq!(retrieved_config.settings.get("test_setting"), 
                   Some(&serde_json::Value::String("test_value".to_string())));
        
        // Test module enabled/disabled
        assert!(!settings_manager.is_module_enabled("test_module").await);
        
        settings_manager.set_module_enabled("test_module".to_string(), true).await.unwrap();
        assert!(settings_manager.is_module_enabled("test_module").await);
        
        settings_manager.shutdown().await;
    }

    #[tokio::test]
    async fn test_settings_manager_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        // Create and configure settings manager
        {
            let mut settings_manager = SettingsManager::new(config_path.clone());
            let mut config = TotthoConfig::default();
            config.startup.restore_session = false;
            config.logging.level = "debug".to_string();
            
            settings_manager.initialize(config).await.unwrap();
            settings_manager.save_config().await.unwrap();
            settings_manager.shutdown().await;
        }
        
        // Verify configuration was saved and can be loaded
        {
            let mut settings_manager = SettingsManager::new(config_path.clone());
            let default_config = TotthoConfig::default();
            settings_manager.initialize(default_config).await.unwrap();
            
            settings_manager.reload_config().await.unwrap();
            
            let loaded_config = settings_manager.get_config().await;
            assert_eq!(loaded_config.startup.restore_session, false);
            assert_eq!(loaded_config.logging.level, "debug");
            
            settings_manager.shutdown().await;
        }
    }

    #[tokio::test]
    async fn test_settings_manager_validation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let mut settings_manager = SettingsManager::new(config_path.clone());
        let config = TotthoConfig::default();
        
        settings_manager.initialize(config).await.unwrap();
        
        // Test invalid configuration update
        let result = settings_manager.update_config(|config| {
            config.logging.level = "invalid_level".to_string();
            Ok(())
        }).await;
        
        assert!(result.is_err());
        
        // Verify configuration wasn't changed
        let current_config = settings_manager.get_config().await;
        assert_eq!(current_config.logging.level, "info"); // Should still be default
        
        settings_manager.shutdown().await;
    }

    #[tokio::test]
    async fn test_settings_manager_reset_to_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let mut settings_manager = SettingsManager::new(config_path.clone());
        let mut config = TotthoConfig::default();
        config.startup.restore_session = false;
        config.logging.level = "debug".to_string();
        
        settings_manager.initialize(config).await.unwrap();
        
        // Verify non-default values
        let current_config = settings_manager.get_config().await;
        assert_eq!(current_config.startup.restore_session, false);
        assert_eq!(current_config.logging.level, "debug");
        
        // Reset to defaults
        settings_manager.reset_to_defaults().await.unwrap();
        
        // Verify reset worked
        let reset_config = settings_manager.get_config().await;
        let default_config = TotthoConfig::default();
        assert_eq!(reset_config.startup.restore_session, default_config.startup.restore_session);
        assert_eq!(reset_config.logging.level, default_config.logging.level);
        
        settings_manager.shutdown().await;
    }

    #[tokio::test]
    async fn test_settings_change_notifications() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let mut settings_manager = SettingsManager::new(config_path.clone());
        let config = TotthoConfig::default();
        
        settings_manager.initialize(config).await.unwrap();
        
        // Set up notification tracking
        let notification_received = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let notification_received_clone = notification_received.clone();
        
        let callback: SettingsChangeCallback = Arc::new(move |_previous, _current| {
            notification_received_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });
        
        settings_manager.subscribe_to_changes(callback).await;
        
        // Make a change
        settings_manager.update_config(|config| {
            config.startup.restore_session = false;
            Ok(())
        }).await.unwrap();
        
        // Give a small delay for async notification
        sleep(Duration::from_millis(10)).await;
        
        // Verify notification was received
        assert!(notification_received.load(std::sync::atomic::Ordering::SeqCst));
        
        settings_manager.shutdown().await;
    }

    #[test]
    fn test_tottho_config_validation() {
        let mut config = TotthoConfig::default();
        
        // Valid configuration should pass
        assert!(config.validate().is_ok());
        
        // Invalid startup timeout
        config.startup.startup_timeout_ms = 0;
        assert!(config.validate().is_err());
        config.startup.startup_timeout_ms = 5000;
        
        // Invalid log level
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
        config.logging.level = "info".to_string();
        
        // Invalid log file size
        config.logging.max_file_size_mb = 0;
        assert!(config.validate().is_err());
        config.logging.max_file_size_mb = 10;
        
        // Invalid window size
        config.workspace.default_window_size.width = 100;
        assert!(config.validate().is_err());
        config.workspace.default_window_size.width = 1200;
        
        config.workspace.default_window_size.height = 100;
        assert!(config.validate().is_err());
        config.workspace.default_window_size.height = 800;
        
        // Should be valid again
        assert!(config.validate().is_ok());
    }

    #[tokio::test]
    async fn test_tottho_config_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let mut config = TotthoConfig::default();
        config.startup.restore_session = false;
        config.logging.level = "debug".to_string();
        
        // Save configuration
        config.save_to_file(&config_path).await.unwrap();
        
        // Verify file exists
        assert!(config_path.exists());
        
        // Load configuration
        let loaded_config = TotthoConfig::load_from_file(&config_path).await.unwrap();
        assert_eq!(loaded_config.startup.restore_session, false);
        assert_eq!(loaded_config.logging.level, "debug");
        
        // Test loading non-existent file (should return defaults)
        let non_existent_path = temp_dir.path().join("non_existent.toml");
        let default_config = TotthoConfig::load_from_file(&non_existent_path).await.unwrap();
        assert_eq!(default_config.startup.restore_session, TotthoConfig::default().startup.restore_session);
    }

    #[test]
    fn test_tottho_config_merge() {
        let mut base_config = TotthoConfig::default();
        base_config.startup.restore_session = true;
        base_config.logging.level = "info".to_string();
        
        let mut other_config = TotthoConfig::default();
        other_config.startup.restore_session = false;
        other_config.logging.level = "debug".to_string();
        other_config.workspace.default_layout = "custom".to_string();
        
        base_config.merge(other_config);
        
        assert_eq!(base_config.startup.restore_session, false);
        assert_eq!(base_config.logging.level, "debug");
        assert_eq!(base_config.workspace.default_layout, "custom");
    }

    #[test]
    fn test_module_config_operations() {
        let mut config = TotthoConfig::default();
        
        // Test getting non-existent module config (should return default)
        let module_config = config.get_module_config("non_existent");
        assert!(module_config.enabled);
        assert!(module_config.settings.is_empty());
        
        // Test setting module config
        let mut custom_config = ModuleConfig::default();
        custom_config.enabled = false;
        custom_config.settings.insert("key".to_string(), serde_json::Value::String("value".to_string()));
        
        config.set_module_config("test_module".to_string(), custom_config.clone());
        
        let retrieved_config = config.get_module_config("test_module");
        assert_eq!(retrieved_config.enabled, false);
        assert_eq!(retrieved_config.settings.get("key"), Some(&serde_json::Value::String("value".to_string())));
        
        // Test module enabled check
        assert!(!config.is_module_enabled("test_module"));
        assert!(config.is_module_enabled("non_existent_module")); // Should default to enabled
    }

    #[tokio::test]
    async fn test_configuration_auto_reload() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        // Create initial configuration file
        let initial_config = TotthoConfig::default();
        initial_config.save_to_file(&config_path).await.unwrap();
        
        let mut settings_manager = SettingsManager::new(config_path.clone());
        settings_manager.initialize(initial_config).await.unwrap();
        
        // Set up notification tracking for auto-reload
        let reload_detected = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let reload_detected_clone = reload_detected.clone();
        
        let callback: SettingsChangeCallback = Arc::new(move |_previous, current| {
            if current.logging.level == "debug" {
                reload_detected_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });
        
        settings_manager.subscribe_to_changes(callback).await;
        
        // Modify configuration file externally
        let mut modified_config = TotthoConfig::default();
        modified_config.logging.level = "debug".to_string();
        modified_config.save_to_file(&config_path).await.unwrap();
        
        // Wait for file watcher to detect change and reload
        for _ in 0..50 { // Wait up to 5 seconds
            sleep(Duration::from_millis(100)).await;
            if reload_detected.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
        }
        
        // Verify configuration was auto-reloaded
        let current_config = settings_manager.get_config().await;
        assert_eq!(current_config.logging.level, "debug");
        assert!(reload_detected.load(std::sync::atomic::Ordering::SeqCst));
        
        settings_manager.shutdown().await;
    }

    #[tokio::test]
    async fn test_configuration_error_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        // Test loading corrupted configuration file
        tokio::fs::write(&config_path, "invalid toml content [[[").await.unwrap();
        
        let result = TotthoConfig::load_from_file(&config_path).await;
        assert!(result.is_err());
        
        // Test recovery by loading defaults when file is corrupted
        let mut settings_manager = SettingsManager::new(config_path.clone());
        let default_config = TotthoConfig::default();
        settings_manager.initialize(default_config.clone()).await.unwrap();
        
        // Verify defaults are used
        let current_config = settings_manager.get_config().await;
        assert_eq!(current_config.startup.restore_session, default_config.startup.restore_session);
        assert_eq!(current_config.logging.level, default_config.logging.level);
        
        // Test recovery from validation errors
        let result = settings_manager.update_config(|config| {
            config.startup.startup_timeout_ms = 0; // Invalid value
            Ok(())
        }).await;
        assert!(result.is_err());
        
        // Verify configuration wasn't corrupted by failed update
        let current_config = settings_manager.get_config().await;
        assert_ne!(current_config.startup.startup_timeout_ms, 0);
        
        settings_manager.shutdown().await;
    }

    #[tokio::test]
    async fn test_settings_synchronization() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let mut settings_manager = SettingsManager::new(config_path.clone());
        let config = TotthoConfig::default();
        settings_manager.initialize(config).await.unwrap();
        
        // Test concurrent access to settings
        let settings_manager = Arc::new(settings_manager);
        let mut handles = Vec::new();
        
        // Spawn multiple tasks that update different parts of the configuration
        for i in 0..10 {
            let settings_manager_clone = settings_manager.clone();
            let handle = tokio::spawn(async move {
                let module_name = format!("module_{}", i);
                let mut module_config = ModuleConfig::default();
                module_config.enabled = i % 2 == 0;
                module_config.settings.insert(
                    "test_key".to_string(), 
                    serde_json::Value::Number(serde_json::Number::from(i))
                );
                
                settings_manager_clone.update_module_config(module_name, module_config).await
            });
            handles.push(handle);
        }
        
        // Wait for all updates to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        
        // Verify all modules were configured correctly
        for i in 0..10 {
            let module_name = format!("module_{}", i);
            let module_config = settings_manager.get_module_config(&module_name).await;
            assert_eq!(module_config.enabled, i % 2 == 0);
            assert_eq!(
                module_config.settings.get("test_key"),
                Some(&serde_json::Value::Number(serde_json::Number::from(i)))
            );
        }
        
        // Test concurrent read access
        let mut read_handles = Vec::new();
        for _ in 0..20 {
            let settings_manager_clone = settings_manager.clone();
            let handle = tokio::spawn(async move {
                settings_manager_clone.get_config().await
            });
            read_handles.push(handle);
        }
        
        // All reads should succeed
        for handle in read_handles {
            let config = handle.await.unwrap();
            assert_eq!(config.logging.level, "info"); // Default value
        }
        
        Arc::try_unwrap(settings_manager).unwrap().shutdown().await;
    }

    #[tokio::test]
    async fn test_default_value_handling() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("non_existent_config.toml");
        
        // Test loading configuration from non-existent file (should use defaults)
        let loaded_config = TotthoConfig::load_from_file(&config_path).await.unwrap();
        
        // Verify all values are defaults
        let default_config = TotthoConfig::default();
        assert_eq!(loaded_config.startup.restore_session, default_config.startup.restore_session);
        assert_eq!(loaded_config.startup.startup_timeout_ms, default_config.startup.startup_timeout_ms);
        assert_eq!(loaded_config.logging.level, default_config.logging.level);
        assert_eq!(loaded_config.logging.log_to_file, default_config.logging.log_to_file);
        assert_eq!(loaded_config.workspace.default_layout, default_config.workspace.default_layout);
        
        // Test settings manager with default configuration
        let mut settings_manager = SettingsManager::new(config_path.clone());
        settings_manager.initialize(loaded_config).await.unwrap();
        
        // Verify defaults are properly handled in settings manager
        let current_config = settings_manager.get_config().await;
        assert_eq!(current_config.startup.startup_timeout_ms, default_config.startup.startup_timeout_ms);
        assert_eq!(current_config.logging.level, default_config.logging.level);
        
        // Test module defaults
        let non_existent_module = settings_manager.get_module_config("non_existent").await;
        assert!(non_existent_module.enabled); // Should default to enabled
        assert!(non_existent_module.settings.is_empty());
        
        // Test that is_module_enabled returns true for non-existent modules (default behavior)
        assert!(settings_manager.is_module_enabled("non_existent_module").await);
        
        // Test creating a new config uses defaults
        let new_config = TotthoConfig::new();
        assert_eq!(new_config.startup.restore_session, default_config.startup.restore_session);
        assert_eq!(new_config.logging.level, default_config.logging.level);
        
        settings_manager.shutdown().await;
    }

    #[tokio::test]
    async fn test_configuration_persistence_edge_cases() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nested/deep/test_config.toml");
        
        // Test saving to non-existent directory (should create directories)
        let config = TotthoConfig::default();
        config.save_to_file(&config_path).await.unwrap();
        
        assert!(config_path.exists());
        assert!(config_path.parent().unwrap().exists());
        
        // Test loading and verifying the saved configuration
        let loaded_config = TotthoConfig::load_from_file(&config_path).await.unwrap();
        assert_eq!(loaded_config.startup.restore_session, config.startup.restore_session);
        
        // Test saving to read-only directory (should fail gracefully)
        let readonly_dir = temp_dir.path().join("readonly");
        tokio::fs::create_dir(&readonly_dir).await.unwrap();
        
        // Make directory read-only on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&readonly_dir).await.unwrap().permissions();
            perms.set_mode(0o444); // Read-only
            tokio::fs::set_permissions(&readonly_dir, perms).await.unwrap();
            
            let readonly_config_path = readonly_dir.join("config.toml");
            let result = config.save_to_file(&readonly_config_path).await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_settings_validation_comprehensive() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let mut settings_manager = SettingsManager::new(config_path.clone());
        let config = TotthoConfig::default();
        settings_manager.initialize(config).await.unwrap();
        
        // Test validation of various invalid configurations
        let invalid_configs = vec![
            // Invalid startup timeout
            |config: &mut TotthoConfig| config.startup.startup_timeout_ms = 0,
            // Invalid log level
            |config: &mut TotthoConfig| config.logging.level = "invalid_level".to_string(),
            // Invalid log file size
            |config: &mut TotthoConfig| config.logging.max_file_size_mb = 0,
            // Invalid window width
            |config: &mut TotthoConfig| config.workspace.default_window_size.width = 100,
            // Invalid window height
            |config: &mut TotthoConfig| config.workspace.default_window_size.height = 100,
        ];
        
        for (i, invalid_config_fn) in invalid_configs.into_iter().enumerate() {
            let result = settings_manager.update_config(|config| {
                invalid_config_fn(config);
                Ok(())
            }).await;
            
            assert!(result.is_err(), "Invalid config {} should have failed validation", i);
            
            // Verify configuration wasn't changed
            let current_config = settings_manager.get_config().await;
            assert!(current_config.validate().is_ok(), "Configuration should still be valid after failed update {}", i);
        }
        
        // Test that validation is called during initialization
        let mut invalid_config = TotthoConfig::default();
        invalid_config.logging.level = "invalid".to_string();
        
        let result = settings_manager.update_config(|config| {
            *config = invalid_config;
            Ok(())
        }).await;
        assert!(result.is_err());
        
        settings_manager.shutdown().await;
    }

    #[tokio::test]
    async fn test_auto_save_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let mut settings_manager = SettingsManager::new(config_path.clone());
        let mut config = TotthoConfig::default();
        config.workspace.auto_save_state = true;
        config.workspace.auto_save_interval_sec = 1; // 1 second for testing
        
        settings_manager.initialize(config).await.unwrap();
        
        // Make a configuration change
        settings_manager.update_config(|config| {
            config.startup.restore_session = false;
            Ok(())
        }).await.unwrap();
        
        // Wait for auto-save to trigger (should happen within 1 second + some buffer)
        sleep(Duration::from_millis(1500)).await;
        
        // Verify configuration was auto-saved by loading from file
        let saved_config = TotthoConfig::load_from_file(&config_path).await.unwrap();
        assert_eq!(saved_config.startup.restore_session, false);
        
        settings_manager.shutdown().await;
    }
}