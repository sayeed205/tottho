//! Error types and handling for the core module
//! 
//! Defines comprehensive error types with context and recovery suggestions.

use thiserror::Error;

/// Result type alias for core operations
pub type Result<T> = std::result::Result<T, CoreError>;

/// Core error types for the Tottho application
#[derive(Debug, Error)]
pub enum CoreError {
    /// Initialization errors
    #[error("Initialization failed: {reason}")]
    InitializationFailed { reason: String },

    /// Module-related errors
    #[error("Module '{name}' already registered")]
    ModuleAlreadyRegistered { name: String },

    #[error("Module '{module}' not found")]
    ModuleNotFound { module: String },

    #[error("Module '{module}' initialization failed: {error}")]
    ModuleInitializationFailed { module: String, error: String },

    #[error("Invalid module interface: {reason}")]
    InvalidModuleInterface { reason: String },

    #[error("Circular dependency detected for module '{module}'")]
    CircularDependency { module: String },

    #[error("Module '{module}' has dependents: {dependents:?}")]
    ModuleHasDependents { module: String, dependents: Vec<String> },

    #[error("Module '{module}' dependency '{dependency}' is not ready")]
    ModuleDependencyNotReady { module: String, dependency: String },

    /// Event bus errors
    #[error("Event bus is not running")]
    EventBusNotRunning,

    #[error("Event handler error: {handler} - {error}")]
    EventHandlerError { handler: String, error: String },

    /// State management errors
    #[error("State error: {0}")]
    StateError(String),

    #[error("State corruption detected: {reason}")]
    StateCorruption { reason: String },

    #[error("State persistence failed: {reason}")]
    StatePersistenceFailed { reason: String },

    #[error("State loading failed: {reason}")]
    StateLoadingFailed { reason: String },

    /// Configuration errors
    #[error("Configuration error: {reason}")]
    ConfigurationError { reason: String },

    #[error("Configuration file not found: {path}")]
    ConfigurationFileNotFound { path: String },

    #[error("Invalid configuration format: {reason}")]
    InvalidConfigurationFormat { reason: String },

    /// GPUI integration errors
    #[error("GPUI error: {reason}")]
    GpuiError { reason: String },

    #[error("Application context error: {reason}")]
    AppContextError { reason: String },

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialization(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDeserialization(#[from] toml::de::Error),

    /// Generic errors
    #[error("Internal error: {reason}")]
    Internal { reason: String },

    #[error("Operation timeout: {operation}")]
    Timeout { operation: String },

    #[error("Resource not available: {resource}")]
    ResourceUnavailable { resource: String },
}

impl CoreError {
    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            CoreError::InitializationFailed { reason } => {
                format!("Application failed to start: {}. Please try restarting the application.", reason)
            }
            CoreError::ModuleInitializationFailed { module, error } => {
                format!("Module '{}' failed to load: {}. Some features may not be available.", module, error)
            }
            CoreError::StateError(reason) => {
                format!("State management error: {}. Please try again.", reason)
            }
            CoreError::StateCorruption { reason } => {
                format!("Application settings are corrupted: {}. Settings will be reset to defaults.", reason)
            }
            CoreError::ConfigurationError { reason } => {
                format!("Configuration error: {}. Please check your settings.", reason)
            }
            CoreError::ConfigurationFileNotFound { path } => {
                format!("Configuration file not found: {}. Default settings will be used.", path)
            }
            CoreError::StatePersistenceFailed { reason } => {
                format!("Failed to save application state: {}. Your changes may not be preserved.", reason)
            }
            CoreError::Timeout { operation } => {
                format!("Operation timed out: {}. Please try again.", operation)
            }
            _ => "An unexpected error occurred. Please try again or restart the application.".to_string(),
        }
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            CoreError::InitializationFailed { .. } => ErrorSeverity::Critical,
            CoreError::ModuleInitializationFailed { .. } => ErrorSeverity::High,
            CoreError::StateError(_) => ErrorSeverity::Medium,
            CoreError::StateCorruption { .. } => ErrorSeverity::High,
            CoreError::CircularDependency { .. } => ErrorSeverity::High,
            CoreError::ConfigurationError { .. } => ErrorSeverity::Medium,
            CoreError::EventBusNotRunning => ErrorSeverity::Medium,
            CoreError::StatePersistenceFailed { .. } => ErrorSeverity::Medium,
            CoreError::Timeout { .. } => ErrorSeverity::Low,
            CoreError::ResourceUnavailable { .. } => ErrorSeverity::Low,
            _ => ErrorSeverity::Medium,
        }
    }

    /// Get suggested recovery actions
    pub fn recovery_actions(&self) -> Vec<RecoveryAction> {
        match self {
            CoreError::InitializationFailed { .. } => vec![
                RecoveryAction::RestartApplication,
                RecoveryAction::CheckSystemRequirements,
                RecoveryAction::ContactSupport,
            ],
            CoreError::ModuleInitializationFailed { .. } => vec![
                RecoveryAction::RestartApplication,
                RecoveryAction::CheckConfiguration,
                RecoveryAction::DisableModule,
            ],
            CoreError::StateCorruption { .. } => vec![
                RecoveryAction::ResetToDefaults,
                RecoveryAction::RestoreFromBackup,
                RecoveryAction::RestartApplication,
            ],
            CoreError::ConfigurationError { .. } => vec![
                RecoveryAction::CheckConfiguration,
                RecoveryAction::ResetToDefaults,
                RecoveryAction::EditConfiguration,
            ],
            CoreError::StatePersistenceFailed { .. } => vec![
                RecoveryAction::CheckDiskSpace,
                RecoveryAction::CheckPermissions,
                RecoveryAction::RetryOperation,
            ],
            _ => vec![RecoveryAction::RetryOperation, RecoveryAction::RestartApplication],
        }
    }

    /// Create an initialization error
    pub fn initialization_failed(reason: impl Into<String>) -> Self {
        CoreError::InitializationFailed {
            reason: reason.into(),
        }
    }

    /// Create a module error
    pub fn module_error(module: impl Into<String>, error: impl Into<String>) -> Self {
        CoreError::ModuleInitializationFailed {
            module: module.into(),
            error: error.into(),
        }
    }

    /// Create a state corruption error
    pub fn state_corruption(reason: impl Into<String>) -> Self {
        CoreError::StateCorruption {
            reason: reason.into(),
        }
    }

    /// Create a configuration error
    pub fn configuration_error(reason: impl Into<String>) -> Self {
        CoreError::ConfigurationError {
            reason: reason.into(),
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Critical errors that prevent application startup
    Critical,
    /// High severity errors that significantly impact functionality
    High,
    /// Medium severity errors that may impact some features
    Medium,
    /// Low severity errors that have minimal impact
    Low,
}

/// Recovery actions that can be suggested to users
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Restart the application
    RestartApplication,
    /// Check system requirements
    CheckSystemRequirements,
    /// Contact support
    ContactSupport,
    /// Check configuration files
    CheckConfiguration,
    /// Edit configuration
    EditConfiguration,
    /// Disable problematic module
    DisableModule,
    /// Reset settings to defaults
    ResetToDefaults,
    /// Restore from backup
    RestoreFromBackup,
    /// Check available disk space
    CheckDiskSpace,
    /// Check file permissions
    CheckPermissions,
    /// Retry the failed operation
    RetryOperation,
}

/// Convert anyhow errors to CoreError
impl From<anyhow::Error> for CoreError {
    fn from(error: anyhow::Error) -> Self {
        CoreError::Internal {
            reason: error.to_string(),
        }
    }
}