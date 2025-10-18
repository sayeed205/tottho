//! UI System Module
//! 
//! This module provides the UI system for Tottho, implementing a modular, theme-aware
//! interface that follows Zed's design principles. It includes composable panels,
//! resizable panes, keyboard-driven navigation, and a unified command palette.

pub mod manager;
pub mod theme_engine;
pub mod layout_engine;
pub mod panel_manager;
pub mod command_palette;
pub mod keyboard_navigator;
pub mod component_registry;
pub mod workspace;
pub mod error;

// Re-export main types
pub use manager::UIManager;
pub use theme_engine::{ThemeEngine, ThemeAware, AppearanceMode};
pub use layout_engine::{LayoutEngine, CenterLayoutMode, PaneGroup, PaneGroupChild};
pub use panel_manager::{PanelManager, Panel};
pub use command_palette::{CommandPalette, Command, CommandInterceptor};
pub use keyboard_navigator::KeyboardNavigator;
pub use component_registry::{ComponentRegistry, UIComponent, ComponentState};
pub use workspace::TotthoWorkspace;
pub use error::{UIError, UIResult};

// Re-export common UI types
pub use gpui::{
    Entity, Context, App, AppContext, WindowHandle, FocusHandle, 
    IntoElement, Render, Styled, Pixels, px, Hsla, Axis, 
    KeyBinding
};

// Re-export layout types
pub use layout_engine::DockPosition;

/// UI System initialization and coordination
pub struct UISystem {
    manager: UIManager,
}

impl UISystem {
    /// Create a new UI system instance
    pub fn new(cx: &mut App) -> anyhow::Result<Self> {
        let manager = UIManager::new(cx)?;
        Ok(Self { manager })
    }
    
    /// Initialize the UI system
    pub fn initialize(&self, cx: &mut App) -> anyhow::Result<()> {
        self.manager.initialize(cx)
            .map_err(|e| anyhow::anyhow!("UI initialization failed: {}", e))
    }
    
    /// Get the UI manager
    pub fn manager(&self) -> &UIManager {
        &self.manager
    }
}