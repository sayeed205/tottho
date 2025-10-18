//! UI Error Types
//! 
//! Defines error types specific to the UI system, following the error handling
//! patterns established in the core system.

use thiserror::Error;

/// UI system error types
#[derive(Debug, Error)]
pub enum UIError {
    #[error("Theme error: {0}")]
    ThemeError(String),
    
    #[error("Layout error: {0}")]
    LayoutError(String),
    
    #[error("Component error in {component}: {error}")]
    ComponentError { component: String, error: String },
    
    #[error("Command palette error: {0}")]
    CommandPaletteError(String),
    
    #[error("Panel error: {0}")]
    PanelError(String),
    
    #[error("Keyboard navigation error: {0}")]
    KeyboardError(String),
    
    #[error("Rendering error: {0}")]
    RenderingError(String),
    
    #[error("Event handling error: {0}")]
    EventError(String),
}

impl UIError {
    /// Get a user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            UIError::ThemeError(msg) => format!("Theme system error: {}", msg),
            UIError::LayoutError(msg) => format!("Layout system error: {}", msg),
            UIError::ComponentError { component, error } => {
                format!("Component '{}' error: {}", component, error)
            }
            UIError::CommandPaletteError(msg) => format!("Command palette error: {}", msg),
            UIError::PanelError(msg) => format!("Panel system error: {}", msg),
            UIError::KeyboardError(msg) => format!("Keyboard navigation error: {}", msg),
            UIError::RenderingError(msg) => format!("UI rendering error: {}", msg),
            UIError::EventError(msg) => format!("UI event handling error: {}", msg),
        }
    }
}

/// Result type for UI operations
pub type UIResult<T> = Result<T, UIError>;