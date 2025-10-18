//! UI Manager
//! 
//! Central coordinator for all UI systems in Tottho. Manages the lifecycle
//! and coordination between theme engine, layout engine, panel manager,
//! command palette, and other UI components.

use std::sync::{Arc, Mutex};
use gpui::{App, Entity, AppContext};
use crate::ui::{
    ThemeEngine, LayoutEngine, PanelManager, CommandPalette,
    ComponentRegistry, KeyboardNavigator, TotthoWorkspace,
    UIError, UIResult, UIComponent
};

/// Central UI system manager
pub struct UIManager {
    theme_engine: Arc<ThemeEngine>,
    layout_engine: Arc<LayoutEngine>,
    panel_manager: Arc<Mutex<PanelManager>>,
    command_palette: Arc<Mutex<CommandPalette>>,
    component_registry: Arc<Mutex<ComponentRegistry>>,
    keyboard_navigator: Arc<Mutex<KeyboardNavigator>>,
}

impl UIManager {
    /// Create a new UI manager instance
    pub fn new(cx: &mut App) -> UIResult<Self> {
        let theme_engine = Arc::new(ThemeEngine::new(cx)?);
        let layout_engine = Arc::new(LayoutEngine::new(cx)?);
        let panel_manager = Arc::new(Mutex::new(PanelManager::new(cx)?));
        let command_palette = Arc::new(Mutex::new(CommandPalette::new(cx)?));
        let component_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
        let keyboard_navigator = Arc::new(Mutex::new(KeyboardNavigator::new(cx)?));
        
        Ok(Self {
            theme_engine,
            layout_engine,
            panel_manager,
            command_palette,
            component_registry,
            keyboard_navigator,
        })
    }
    
    /// Initialize the UI system
    pub fn initialize(&self, cx: &mut App) -> UIResult<()> {
        // Initialize theme engine first
        self.theme_engine.initialize(cx)
            .map_err(|e| UIError::ThemeError(e.to_string()))?;
        
        // Initialize layout engine
        self.layout_engine.initialize(cx)
            .map_err(|e| UIError::LayoutError(e.to_string()))?;
        
        // Initialize panel manager
        self.panel_manager.lock().unwrap().initialize(cx)
            .map_err(|e| UIError::PanelError(e.to_string()))?;
        
        // Initialize command palette
        self.command_palette.lock().unwrap().initialize(cx)
            .map_err(|e| UIError::CommandPaletteError(e.to_string()))?;
        
        // Initialize keyboard navigator
        self.keyboard_navigator.lock().unwrap().initialize(cx)
            .map_err(|e| UIError::KeyboardError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Create a new workspace instance
    pub fn create_workspace(&self, cx: &mut App) -> UIResult<Entity<TotthoWorkspace>> {
        let workspace = cx.new(|cx| {
            TotthoWorkspace::new(
                self.theme_engine.clone(),
                self.layout_engine.clone(),
                self.panel_manager.clone(),
                self.command_palette.clone(),
                self.keyboard_navigator.clone(),
                cx,
            )
        });
        
        Ok(workspace)
    }
    
    /// Register a UI component
    pub fn register_component<T: UIComponent>(&self, component: T) -> UIResult<()> {
        self.component_registry.lock().unwrap().register_component(component)
            .map_err(|e| UIError::ComponentError {
                component: std::any::type_name::<T>().to_string(),
                error: e.to_string(),
            })
    }
    
    /// Get the theme engine
    pub fn theme_engine(&self) -> &Arc<ThemeEngine> {
        &self.theme_engine
    }
    
    /// Get the layout engine
    pub fn layout_engine(&self) -> &Arc<LayoutEngine> {
        &self.layout_engine
    }
    
    /// Get the panel manager
    pub fn panel_manager(&self) -> &Arc<Mutex<PanelManager>> {
        &self.panel_manager
    }
    
    /// Get the command palette
    pub fn command_palette(&self) -> &Arc<Mutex<CommandPalette>> {
        &self.command_palette
    }
    
    /// Get the component registry
    pub fn component_registry(&self) -> &Arc<Mutex<ComponentRegistry>> {
        &self.component_registry
    }
    
    /// Get the keyboard navigator
    pub fn keyboard_navigator(&self) -> &Arc<Mutex<KeyboardNavigator>> {
        &self.keyboard_navigator
    }
}