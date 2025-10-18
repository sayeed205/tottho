//! Error types and handling for the core module
//! 
//! Defines comprehensive error types with context and recovery suggestions.

use thiserror::Error;
use anyhow::Context;
use std::fmt;
use tracing::{error, warn, info};

/// Result type alias for core operations
pub type Result<T> = std::result::Result<T, CoreError>;

/// Core error types for the Tottho application
#[derive(Debug, Error, Clone)]
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
    Io(String),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("TOML serialization error: {0}")]
    TomlSerialization(String),

    #[error("TOML deserialization error: {0}")]
    TomlDeserialization(String),

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

/// Error categories for different recovery strategies
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// Initialization and startup errors
    Initialization,
    /// Module management errors
    Module,
    /// Event system errors
    Event,
    /// State management errors
    State,
    /// Configuration errors
    Configuration,
    /// UI/GPUI errors
    Ui,
    /// I/O and persistence errors
    Io,
    /// Internal system errors
    Internal,
}

impl CoreError {
    /// Get the error category for recovery strategy selection
    pub fn category(&self) -> ErrorCategory {
        match self {
            CoreError::InitializationFailed { .. } => ErrorCategory::Initialization,
            CoreError::ModuleAlreadyRegistered { .. }
            | CoreError::ModuleNotFound { .. }
            | CoreError::ModuleInitializationFailed { .. }
            | CoreError::InvalidModuleInterface { .. }
            | CoreError::CircularDependency { .. }
            | CoreError::ModuleHasDependents { .. }
            | CoreError::ModuleDependencyNotReady { .. } => ErrorCategory::Module,
            CoreError::EventBusNotRunning
            | CoreError::EventHandlerError { .. } => ErrorCategory::Event,
            CoreError::StateError(_)
            | CoreError::StateCorruption { .. }
            | CoreError::StatePersistenceFailed { .. }
            | CoreError::StateLoadingFailed { .. } => ErrorCategory::State,
            CoreError::ConfigurationError { .. }
            | CoreError::ConfigurationFileNotFound { .. }
            | CoreError::InvalidConfigurationFormat { .. } => ErrorCategory::Configuration,
            CoreError::GpuiError { .. }
            | CoreError::AppContextError { .. } => ErrorCategory::Ui,
            CoreError::Io(_) => ErrorCategory::Io,
            CoreError::Serialization(_)
            | CoreError::TomlSerialization(_)
            | CoreError::TomlDeserialization(_)
            | CoreError::Internal { .. }
            | CoreError::Timeout { .. }
            | CoreError::ResourceUnavailable { .. } => ErrorCategory::Internal,
        }
    }

    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            CoreError::InitializationFailed { .. } => false,
            CoreError::CircularDependency { .. } => false,
            CoreError::StateCorruption { .. } => true, // Can reset to defaults
            CoreError::ConfigurationError { .. } => true,
            CoreError::StatePersistenceFailed { .. } => true,
            CoreError::Timeout { .. } => true,
            CoreError::ResourceUnavailable { .. } => true,
            CoreError::EventBusNotRunning => true,
            _ => false,
        }
    }

    /// Check if the error requires immediate user attention
    pub fn requires_user_attention(&self) -> bool {
        matches!(
            self.severity(),
            ErrorSeverity::Critical | ErrorSeverity::High
        )
    }

    /// Add context to an error using anyhow
    pub fn with_context<C>(self, context: C) -> anyhow::Error
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        anyhow::Error::from(self).context(context)
    }

    /// Create an error with context chain
    pub fn chain_context<C>(error: anyhow::Error, context: C) -> anyhow::Error
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        error.context(context)
    }
}

/// Error context information for debugging and logging
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub component: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub additional_info: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>, component: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            component: component.into(),
            timestamp: chrono::Utc::now(),
            additional_info: std::collections::HashMap::new(),
        }
    }

    pub fn with_info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_info.insert(key.into(), value.into());
        self
    }
}

/// Convert anyhow errors to CoreError
impl From<anyhow::Error> for CoreError {
    fn from(error: anyhow::Error) -> Self {
        CoreError::Internal {
            reason: error.to_string(),
        }
    }
}

/// Convert std::io::Error to CoreError
impl From<std::io::Error> for CoreError {
    fn from(error: std::io::Error) -> Self {
        CoreError::Io(error.to_string())
    }
}

/// Convert serde_json::Error to CoreError
impl From<serde_json::Error> for CoreError {
    fn from(error: serde_json::Error) -> Self {
        CoreError::Serialization(error.to_string())
    }
}

/// Convert toml::ser::Error to CoreError
impl From<toml::ser::Error> for CoreError {
    fn from(error: toml::ser::Error) -> Self {
        CoreError::TomlSerialization(error.to_string())
    }
}

/// Convert toml::de::Error to CoreError
impl From<toml::de::Error> for CoreError {
    fn from(error: toml::de::Error) -> Self {
        CoreError::TomlDeserialization(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[test]
    fn test_error_categories() {
        assert_eq!(
            CoreError::InitializationFailed { reason: "test".to_string() }.category(),
            ErrorCategory::Initialization
        );
        assert_eq!(
            CoreError::ModuleNotFound { module: "test".to_string() }.category(),
            ErrorCategory::Module
        );
        assert_eq!(
            CoreError::EventBusNotRunning.category(),
            ErrorCategory::Event
        );
        assert_eq!(
            CoreError::StateError("test".to_string()).category(),
            ErrorCategory::State
        );
        assert_eq!(
            CoreError::ConfigurationError { reason: "test".to_string() }.category(),
            ErrorCategory::Configuration
        );
        assert_eq!(
            CoreError::GpuiError { reason: "test".to_string() }.category(),
            ErrorCategory::Ui
        );
        assert_eq!(
            CoreError::Io("test".to_string()).category(),
            ErrorCategory::Io
        );
    }

    #[test]
    fn test_error_severity() {
        assert_eq!(
            CoreError::InitializationFailed { reason: "test".to_string() }.severity(),
            ErrorSeverity::Critical
        );
        assert_eq!(
            CoreError::ModuleInitializationFailed { 
                module: "test".to_string(), 
                error: "test".to_string() 
            }.severity(),
            ErrorSeverity::High
        );
        assert_eq!(
            CoreError::StateError("test".to_string()).severity(),
            ErrorSeverity::Medium
        );
        assert_eq!(
            CoreError::Timeout { operation: "test".to_string() }.severity(),
            ErrorSeverity::Low
        );
    }

    #[test]
    fn test_error_recoverability() {
        assert!(!CoreError::InitializationFailed { reason: "test".to_string() }.is_recoverable());
        assert!(!CoreError::CircularDependency { module: "test".to_string() }.is_recoverable());
        assert!(CoreError::StateCorruption { reason: "test".to_string() }.is_recoverable());
        assert!(CoreError::ConfigurationError { reason: "test".to_string() }.is_recoverable());
        assert!(CoreError::Timeout { operation: "test".to_string() }.is_recoverable());
    }

    #[test]
    fn test_user_attention_requirement() {
        assert!(CoreError::InitializationFailed { reason: "test".to_string() }.requires_user_attention());
        assert!(CoreError::ModuleInitializationFailed { 
            module: "test".to_string(), 
            error: "test".to_string() 
        }.requires_user_attention());
        assert!(!CoreError::Timeout { operation: "test".to_string() }.requires_user_attention());
    }

    #[test]
    fn test_user_messages() {
        let error = CoreError::InitializationFailed { reason: "GPUI failed".to_string() };
        let message = error.user_message();
        assert!(message.contains("Application failed to start"));
        assert!(message.contains("GPUI failed"));

        let error = CoreError::StateCorruption { reason: "Invalid format".to_string() };
        let message = error.user_message();
        assert!(message.contains("corrupted"));
        assert!(message.contains("reset to defaults"));
    }

    #[test]
    fn test_recovery_actions() {
        let error = CoreError::InitializationFailed { reason: "test".to_string() };
        let actions = error.recovery_actions();
        assert!(actions.contains(&RecoveryAction::RestartApplication));
        assert!(actions.contains(&RecoveryAction::CheckSystemRequirements));

        let error = CoreError::StateCorruption { reason: "test".to_string() };
        let actions = error.recovery_actions();
        assert!(actions.contains(&RecoveryAction::ResetToDefaults));
        assert!(actions.contains(&RecoveryAction::RestoreFromBackup));
    }

    #[test]
    fn test_error_context() {
        let mut context = ErrorContext::new("test_operation", "test_component");
        context = context.with_info("key1", "value1").with_info("key2", "value2");

        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.component, "test_component");
        assert_eq!(context.additional_info.get("key1"), Some(&"value1".to_string()));
        assert_eq!(context.additional_info.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_error_with_context() {
        let error = CoreError::StateError("test".to_string());
        let anyhow_error = error.with_context("Additional context");
        assert!(anyhow_error.to_string().contains("Additional context"));
    }

    #[test]
    fn test_result_ext() {
        let result: Result<i32> = Err(CoreError::StateError("test".to_string()));
        let anyhow_result = result.with_core_context("Operation failed");
        assert!(anyhow_result.is_err());
        assert!(anyhow_result.unwrap_err().to_string().contains("Operation failed"));
    }

    #[tokio::test]
    async fn test_error_handler_creation() {
        let handler = ErrorHandler::new();
        assert!(handler.recovery_strategies.is_empty());
        assert!(handler.notification_sender.is_none());
    }

    #[tokio::test]
    async fn test_error_handler_with_strategies() {
        let mut handler = ErrorHandler::new();
        handler.register_recovery_strategy(ErrorCategory::State, StateRecoveryStrategy);
        
        assert!(handler.recovery_strategies.contains_key(&ErrorCategory::State));
    }

    #[tokio::test]
    async fn test_state_recovery_strategy() {
        let strategy = StateRecoveryStrategy;
        let context = ErrorContext::new("test", "test");
        
        // Test successful recovery
        let error = CoreError::StateCorruption { reason: "test".to_string() };
        let result = strategy.recover(&error, &context).await;
        assert!(result.is_ok());
        
        // Test unsupported error type
        let error = CoreError::InitializationFailed { reason: "test".to_string() };
        let result = strategy.recover(&error, &context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_config_recovery_strategy() {
        let strategy = ConfigRecoveryStrategy;
        let context = ErrorContext::new("test", "test");
        
        // Test successful recovery
        let error = CoreError::ConfigurationFileNotFound { path: "test.toml".to_string() };
        let result = strategy.recover(&error, &context).await;
        assert!(result.is_ok());
        
        // Test unsupported error type
        let error = CoreError::InitializationFailed { reason: "test".to_string() };
        let result = strategy.recover(&error, &context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_module_recovery_strategy() {
        let strategy = ModuleRecoveryStrategy;
        let context = ErrorContext::new("test", "test");
        
        // Test successful recovery
        let error = CoreError::ModuleInitializationFailed { 
            module: "test".to_string(), 
            error: "test".to_string() 
        };
        let result = strategy.recover(&error, &context).await;
        assert!(result.is_ok());
        
        // Test unsupported error type
        let error = CoreError::InitializationFailed { reason: "test".to_string() };
        let result = strategy.recover(&error, &context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_event_recovery_strategy() {
        let strategy = EventRecoveryStrategy;
        let context = ErrorContext::new("test", "test");
        
        // Test successful recovery
        let error = CoreError::EventBusNotRunning;
        let result = strategy.recover(&error, &context).await;
        assert!(result.is_ok());
        
        // Test unsupported error type
        let error = CoreError::InitializationFailed { reason: "test".to_string() };
        let result = strategy.recover(&error, &context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_error_handler_with_recovery() {
        let mut handler = ErrorHandler::new();
        handler.register_recovery_strategy(ErrorCategory::State, StateRecoveryStrategy);
        
        let error = CoreError::StateCorruption { reason: "test".to_string() };
        let context = ErrorContext::new("test", "test");
        
        let result = handler.handle_error(&error, context).await;
        assert!(matches!(result, RecoveryResult::Success));
    }

    #[tokio::test]
    async fn test_error_handler_no_strategy() {
        let handler = ErrorHandler::new();
        
        let error = CoreError::StateCorruption { reason: "test".to_string() };
        let context = ErrorContext::new("test", "test");
        
        let result = handler.handle_error(&error, context).await;
        assert!(matches!(result, RecoveryResult::NoStrategy));
    }

    #[tokio::test]
    async fn test_error_handler_non_recoverable() {
        let handler = ErrorHandler::new();
        
        let error = CoreError::InitializationFailed { reason: "test".to_string() };
        let context = ErrorContext::new("test", "test");
        
        let result = handler.handle_error(&error, context).await;
        assert!(matches!(result, RecoveryResult::Failed(_)));
    }

    #[tokio::test]
    async fn test_global_error_handler_initialization() {
        let handler = initialize_error_handler();
        let handler_guard = handler.read().await;
        
        // Should have default recovery strategies
        assert!(handler_guard.recovery_strategies.contains_key(&ErrorCategory::State));
        assert!(handler_guard.recovery_strategies.contains_key(&ErrorCategory::Configuration));
        assert!(handler_guard.recovery_strategies.contains_key(&ErrorCategory::Module));
        assert!(handler_guard.recovery_strategies.contains_key(&ErrorCategory::Event));
    }

    #[tokio::test]
    async fn test_global_error_handling() {
        // Initialize the global handler
        let _handler = initialize_error_handler();
        
        let error = CoreError::StateCorruption { reason: "test".to_string() };
        let context = ErrorContext::new("test", "test");
        
        let result = handle_error(error, context).await;
        assert!(matches!(result, RecoveryResult::Success));
    }

    #[test]
    fn test_error_notification_creation() {
        let error = CoreError::StateError("test".to_string());
        let context = ErrorContext::new("test", "test");
        let recovery_result = RecoveryResult::Success;
        
        let notification = ErrorNotification {
            error: error.clone(),
            context: context.clone(),
            recovery_result: recovery_result.clone(),
            user_message: error.user_message(),
            recovery_actions: error.recovery_actions(),
            timestamp: chrono::Utc::now(),
        };
        
        assert_eq!(notification.error.category(), ErrorCategory::State);
        assert!(notification.user_message.contains("State management error"));
    }

    #[test]
    fn test_error_from_conversions() {
        // Test std::io::Error conversion
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let core_error: CoreError = io_error.into();
        assert!(matches!(core_error, CoreError::Io(_)));

        // Test serde_json::Error conversion
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let core_error: CoreError = json_error.into();
        assert!(matches!(core_error, CoreError::Serialization(_)));

        // Test anyhow::Error conversion
        let anyhow_error = anyhow::anyhow!("Test error");
        let core_error: CoreError = anyhow_error.into();
        assert!(matches!(core_error, CoreError::Internal { .. }));
    }

    #[test]
    fn test_error_helper_methods() {
        let error = CoreError::initialization_failed("Test reason");
        assert!(matches!(error, CoreError::InitializationFailed { .. }));

        let error = CoreError::module_error("test_module", "Test error");
        assert!(matches!(error, CoreError::ModuleInitializationFailed { .. }));

        let error = CoreError::state_corruption("Test corruption");
        assert!(matches!(error, CoreError::StateCorruption { .. }));

        let error = CoreError::configuration_error("Test config error");
        assert!(matches!(error, CoreError::ConfigurationError { .. }));
    }
}

/// Helper trait for adding context to Results
pub trait CoreResultExt<T> {
    /// Add context to a Result
    fn with_core_context<C>(self, context: C) -> anyhow::Result<T>
    where
        C: fmt::Display + Send + Sync + 'static;

    /// Add context using a closure
    fn with_core_context_lazy<C, F>(self, f: F) -> anyhow::Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<T> CoreResultExt<T> for Result<T> {
    fn with_core_context<C>(self, context: C) -> anyhow::Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        self.map_err(|e| e.with_context(context))
    }

    fn with_core_context_lazy<C, F>(self, f: F) -> anyhow::Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|e| e.with_context(f()))
    }
}

/// Centralized error handler for the application
pub struct ErrorHandler {
    recovery_strategies: std::collections::HashMap<ErrorCategory, Box<dyn RecoveryStrategy + Send + Sync>>,
    notification_sender: Option<tokio::sync::mpsc::UnboundedSender<ErrorNotification>>,
}

impl ErrorHandler {
    /// Create a new error handler
    pub fn new() -> Self {
        Self {
            recovery_strategies: std::collections::HashMap::new(),
            notification_sender: None,
        }
    }

    /// Set the notification sender for user notifications
    pub fn set_notification_sender(&mut self, sender: tokio::sync::mpsc::UnboundedSender<ErrorNotification>) {
        self.notification_sender = Some(sender);
    }

    /// Register a recovery strategy for an error category
    pub fn register_recovery_strategy<S>(&mut self, category: ErrorCategory, strategy: S)
    where
        S: RecoveryStrategy + Send + Sync + 'static,
    {
        self.recovery_strategies.insert(category, Box::new(strategy));
    }

    /// Handle an error with context
    pub async fn handle_error(&self, error: &CoreError, context: ErrorContext) -> RecoveryResult {
        // Log the error with appropriate level
        self.log_error(error, &context);

        // Attempt recovery if possible
        let recovery_result = if error.is_recoverable() {
            self.attempt_recovery(error, &context).await
        } else {
            RecoveryResult::Failed("Error is not recoverable".to_string())
        };

        // Send user notification if required
        if error.requires_user_attention() {
            self.notify_user(error, &context, &recovery_result).await;
        }

        recovery_result
    }

    /// Log error with appropriate severity
    fn log_error(&self, error: &CoreError, context: &ErrorContext) {
        let error_msg = format!(
            "Error in {}.{}: {} | Context: {:?}",
            context.component, context.operation, error, context.additional_info
        );

        match error.severity() {
            ErrorSeverity::Critical => error!("{}", error_msg),
            ErrorSeverity::High => error!("{}", error_msg),
            ErrorSeverity::Medium => warn!("{}", error_msg),
            ErrorSeverity::Low => info!("{}", error_msg),
        }
    }

    /// Attempt to recover from an error
    async fn attempt_recovery(&self, error: &CoreError, context: &ErrorContext) -> RecoveryResult {
        let category = error.category();
        
        if let Some(strategy) = self.recovery_strategies.get(&category) {
            match strategy.recover(error, context).await {
                Ok(()) => RecoveryResult::Success,
                Err(recovery_error) => {
                    warn!("Recovery failed for error {:?}: {}", error, recovery_error);
                    RecoveryResult::Failed(recovery_error)
                }
            }
        } else {
            RecoveryResult::NoStrategy
        }
    }

    /// Send notification to user
    async fn notify_user(&self, error: &CoreError, context: &ErrorContext, recovery: &RecoveryResult) {
        if let Some(sender) = &self.notification_sender {
            let notification = ErrorNotification {
                error: error.clone(),
                context: context.clone(),
                recovery_result: recovery.clone(),
                user_message: error.user_message(),
                recovery_actions: error.recovery_actions(),
                timestamp: chrono::Utc::now(),
            };

            if let Err(e) = sender.send(notification) {
                error!("Failed to send error notification: {}", e);
            }
        }
    }
}

impl Default for ErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Recovery strategy trait for different error categories
#[async_trait::async_trait]
pub trait RecoveryStrategy {
    async fn recover(&self, error: &CoreError, context: &ErrorContext) -> std::result::Result<(), String>;
}

/// Result of a recovery attempt
#[derive(Debug, Clone)]
pub enum RecoveryResult {
    /// Recovery was successful
    Success,
    /// Recovery failed with reason
    Failed(String),
    /// No recovery strategy available
    NoStrategy,
}

/// Error notification for user interface
#[derive(Debug, Clone)]
pub struct ErrorNotification {
    pub error: CoreError,
    pub context: ErrorContext,
    pub recovery_result: RecoveryResult,
    pub user_message: String,
    pub recovery_actions: Vec<RecoveryAction>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Default recovery strategies for common error categories
pub struct DefaultRecoveryStrategies;

impl DefaultRecoveryStrategies {
    /// Create default recovery strategies
    pub fn create_all() -> std::collections::HashMap<ErrorCategory, Box<dyn RecoveryStrategy + Send + Sync>> {
        let mut strategies: std::collections::HashMap<ErrorCategory, Box<dyn RecoveryStrategy + Send + Sync>> = std::collections::HashMap::new();
        
        strategies.insert(ErrorCategory::State, Box::new(StateRecoveryStrategy));
        strategies.insert(ErrorCategory::Configuration, Box::new(ConfigRecoveryStrategy));
        strategies.insert(ErrorCategory::Module, Box::new(ModuleRecoveryStrategy));
        strategies.insert(ErrorCategory::Event, Box::new(EventRecoveryStrategy));
        
        strategies
    }
}

/// State recovery strategy
pub struct StateRecoveryStrategy;

#[async_trait::async_trait]
impl RecoveryStrategy for StateRecoveryStrategy {
    async fn recover(&self, error: &CoreError, _context: &ErrorContext) -> std::result::Result<(), String> {
        match error {
            CoreError::StateCorruption { .. } => {
                info!("Attempting to recover from state corruption by resetting to defaults");
                // In a real implementation, this would reset state to defaults
                Ok(())
            }
            CoreError::StatePersistenceFailed { .. } => {
                info!("Attempting to recover from state persistence failure");
                // In a real implementation, this would retry saving or use alternative storage
                Ok(())
            }
            _ => Err("State recovery strategy cannot handle this error type".to_string()),
        }
    }
}

/// Configuration recovery strategy
pub struct ConfigRecoveryStrategy;

#[async_trait::async_trait]
impl RecoveryStrategy for ConfigRecoveryStrategy {
    async fn recover(&self, error: &CoreError, _context: &ErrorContext) -> std::result::Result<(), String> {
        match error {
            CoreError::ConfigurationFileNotFound { .. } => {
                info!("Attempting to recover from missing configuration by creating defaults");
                // In a real implementation, this would create default configuration
                Ok(())
            }
            CoreError::InvalidConfigurationFormat { .. } => {
                info!("Attempting to recover from invalid configuration format");
                // In a real implementation, this would validate and fix configuration
                Ok(())
            }
            _ => Err("Configuration recovery strategy cannot handle this error type".to_string()),
        }
    }
}

/// Module recovery strategy
pub struct ModuleRecoveryStrategy;

#[async_trait::async_trait]
impl RecoveryStrategy for ModuleRecoveryStrategy {
    async fn recover(&self, error: &CoreError, _context: &ErrorContext) -> std::result::Result<(), String> {
        match error {
            CoreError::ModuleInitializationFailed { module, .. } => {
                warn!("Attempting to recover from module initialization failure: {}", module);
                // In a real implementation, this would try to reinitialize the module
                // or disable it gracefully
                Ok(())
            }
            _ => Err("Module recovery strategy cannot handle this error type".to_string()),
        }
    }
}

/// Event system recovery strategy
pub struct EventRecoveryStrategy;

#[async_trait::async_trait]
impl RecoveryStrategy for EventRecoveryStrategy {
    async fn recover(&self, error: &CoreError, _context: &ErrorContext) -> std::result::Result<(), String> {
        match error {
            CoreError::EventBusNotRunning => {
                info!("Attempting to recover from event bus failure by restarting");
                // In a real implementation, this would restart the event bus
                Ok(())
            }
            _ => Err("Event recovery strategy cannot handle this error type".to_string()),
        }
    }
}

/// Global error handler instance
static ERROR_HANDLER: std::sync::OnceLock<std::sync::Arc<tokio::sync::RwLock<ErrorHandler>>> = std::sync::OnceLock::new();

/// Initialize the global error handler
pub fn initialize_error_handler() -> std::sync::Arc<tokio::sync::RwLock<ErrorHandler>> {
    ERROR_HANDLER.get_or_init(|| {
        let mut handler = ErrorHandler::new();
        
        // Register default recovery strategies
        let strategies = DefaultRecoveryStrategies::create_all();
        for (category, strategy) in strategies {
            handler.recovery_strategies.insert(category, strategy);
        }
        
        std::sync::Arc::new(tokio::sync::RwLock::new(handler))
    }).clone()
}

/// Get the global error handler
pub fn get_error_handler() -> Option<std::sync::Arc<tokio::sync::RwLock<ErrorHandler>>> {
    ERROR_HANDLER.get().cloned()
}

/// Handle an error using the global error handler
pub async fn handle_error(error: CoreError, context: ErrorContext) -> RecoveryResult {
    if let Some(handler) = get_error_handler() {
        let handler = handler.read().await;
        handler.handle_error(&error, context).await
    } else {
        // Fallback logging if handler not initialized
        error!("Error occurred but handler not initialized: {:?}", error);
        RecoveryResult::NoStrategy
    }
}

/// Convenience macro for handling errors with context
#[macro_export]
macro_rules! handle_error {
    ($error:expr, $operation:expr, $component:expr) => {
        $crate::core::error::handle_error(
            $error,
            $crate::core::error::ErrorContext::new($operation, $component)
        ).await
    };
    ($error:expr, $operation:expr, $component:expr, $($key:expr => $value:expr),*) => {
        {
            let mut context = $crate::core::error::ErrorContext::new($operation, $component);
            $(
                context = context.with_info($key, $value);
            )*
            $crate::core::error::handle_error($error, context).await
        }
    };
}