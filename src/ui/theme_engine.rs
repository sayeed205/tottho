//! Theme Engine
//! 
//! Manages theme loading, switching, and application throughout the UI system.
//! Supports Zed theme compatibility and runtime theme switching.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use gpui::{App, Context, Global};
use theme::{Theme, ThemeRegistry, GlobalTheme};
use serde::{Serialize, Deserialize};
use crate::ui::{UIError, UIResult};

/// Theme engine for managing themes and appearance
pub struct ThemeEngine {
    active_theme: Arc<RwLock<Theme>>,
    available_themes: HashMap<String, Theme>,
    appearance_mode: AppearanceMode,
}

/// Appearance mode configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppearanceMode {
    /// Follow system appearance
    System,
    /// Force light mode
    Light,
    /// Force dark mode
    Dark,
}

/// Trait for components that respond to theme changes
pub trait ThemeAware {
    /// Apply a new theme to the component
    fn apply_theme(&mut self, theme: &Theme, cx: &mut Context<Self>) where Self: Sized;
    
    /// Handle theme change notification
    fn theme_changed(&mut self, theme: &Theme, cx: &mut Context<Self>) where Self: Sized {
        self.apply_theme(theme, cx);
    }
}

impl ThemeEngine {
    /// Create a new theme engine
    pub fn new(cx: &mut App) -> UIResult<Self> {
        // For now, create a minimal theme engine that will be properly implemented in task 2.1
        // We'll use a placeholder theme until the theme system is fully implemented
        let placeholder_theme = Self::create_placeholder_theme();
        
        Ok(Self {
            active_theme: Arc::new(RwLock::new(placeholder_theme)),
            available_themes: HashMap::new(),
            appearance_mode: AppearanceMode::System,
        })
    }
    
    /// Create a placeholder theme for initial setup
    fn create_placeholder_theme() -> Theme {
        // This will be replaced with proper theme loading in task 2.1
        // For now, we use a minimal approach that works with the current Zed theme system
        
        // Create a theme registry and try to get the first available theme
        let theme_registry = ThemeRegistry::default();
        let themes = theme_registry.list();
        
        // Use the first theme if available
        if let Some(theme_meta) = themes.first() {
            match theme_registry.get(&theme_meta.name) {
                Ok(theme_arc) => {
                    // Clone the theme from the Arc
                    (*theme_arc).clone()
                }
                Err(_) => {
                    Self::create_minimal_fallback_theme()
                }
            }
        } else {
            Self::create_minimal_fallback_theme()
        }
    }
    
    /// Create a minimal fallback theme when no themes are available
    fn create_minimal_fallback_theme() -> Theme {
        // This is a last resort fallback that should work with any Zed version
        // It will be replaced with proper theme loading in task 2.1
        
        // Instead of trying to create a theme manually, let's try to get any available theme
        // from the registry as a fallback, and if that fails, we'll panic with a helpful message
        let theme_registry = ThemeRegistry::default();
        let themes = theme_registry.list();
        
        // Try to get any theme from the registry
        for theme_meta in themes {
            if let Ok(theme_arc) = theme_registry.get(&theme_meta.name) {
                return (*theme_arc).clone();
            }
        }
        
        // If we can't get any theme, this is a critical error
        panic!("No themes available in ThemeRegistry. This indicates a problem with the Zed theme system setup.");
    }
    
    /// Initialize the theme engine
    pub fn initialize(&self, _cx: &mut App) -> anyhow::Result<()> {
        // TODO: Load Zed themes and detect system appearance
        Ok(())
    }
    
    /// Load Zed themes from the theme system
    pub fn load_zed_themes(&mut self) -> UIResult<()> {
        // TODO: Implement Zed theme loading
        // This will be implemented in task 2.1
        Ok(())
    }
    
    /// Set the active theme
    pub fn set_active_theme(&self, name: &str) -> UIResult<()> {
        if let Some(theme) = self.available_themes.get(name) {
            *self.active_theme.write().unwrap() = theme.clone();
            // TODO: Notify all theme-aware components
            Ok(())
        } else {
            Err(UIError::ThemeError(format!("Theme '{}' not found", name)))
        }
    }
    
    /// Get the current active theme
    pub fn active_theme(&self) -> Theme {
        self.active_theme.read().unwrap().clone()
    }
    
    /// Set appearance mode
    pub fn set_appearance_mode(&mut self, mode: AppearanceMode) -> UIResult<()> {
        self.appearance_mode = mode;
        // TODO: Apply appearance mode changes
        Ok(())
    }
    
    /// Get current appearance mode
    pub fn appearance_mode(&self) -> AppearanceMode {
        self.appearance_mode
    }
    
    /// Validate a theme for compatibility
    pub fn validate_theme(&self, _theme: &Theme) -> UIResult<()> {
        // TODO: Implement theme validation
        Ok(())
    }
}