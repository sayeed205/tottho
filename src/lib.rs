//! Tottho Core Library
//! 
//! This library provides the core functionality for the Tottho database management tool.
//! It follows Zed's architectural patterns and provides a modular, extensible foundation.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod core;
pub mod ui;

// Re-export core types for library users
pub use core::{
    TotthoCore, TotthoApp, EventBus, Event, EventHandler,
    Module, ModuleRegistry, ModuleInfo, ModuleState,
    ApplicationState, WorkspaceLayout, UserPreferences, SessionData,
    CoreError, Result,
    TotthoConfig, StartupConfig, LoggingConfig, WorkspaceConfig, SettingsManager, SettingsChangeCallback, SettingsChangeEvent, SettingsChangeSource,
    WorkspaceManager, GlobalWorkspaceManager, Workspace, WorkspaceId,
    WindowState, PanelInfo, PanelPosition, TabInfo, TabType,
    init_logging, VERSION,
};

// Re-export specific types from submodules
pub use core::event_bus::CoreEvent;
pub use core::state::{ConnectionProfile, QueryHistoryEntry};

// Re-export UI system types
pub use ui::{
    UISystem, UIManager, ThemeEngine, LayoutEngine, PanelManager, 
    CommandPalette, KeyboardNavigator, ComponentRegistry, TotthoWorkspace,
    UIError, UIResult, Panel, UIComponent, ThemeAware, AppearanceMode,
    CenterLayoutMode, DockPosition, ComponentState
};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::core::{
        TotthoCore, TotthoApp, EventBus, Event, EventHandler,
        Module, ModuleRegistry, ApplicationState, CoreError, Result,
        TotthoConfig, SettingsManager, WorkspaceManager, GlobalWorkspaceManager,
    };
    pub use crate::ui::{
        UISystem, UIManager, ThemeEngine, LayoutEngine, PanelManager,
        CommandPalette, KeyboardNavigator, TotthoWorkspace, UIError, UIResult,
        Panel, UIComponent, ThemeAware,
    };
    pub use async_trait::async_trait;
    pub use gpui::{App, AppContext, Context, IntoElement, Render};
    pub use anyhow;
    pub use tokio;
}