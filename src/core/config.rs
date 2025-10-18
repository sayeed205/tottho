//! Configuration management for the Tottho application
//! 
//! Handles loading, validation, and persistence of application configuration.

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

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