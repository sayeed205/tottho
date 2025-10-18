//! Panel Manager
//! 
//! Manages panels and dock system, providing registration, lifecycle management,
//! and dock operations for UI panels.

use std::collections::HashMap;
use gpui::{App, Context, FocusHandle, IntoElement, AnyElement};
use crate::ui::{UIError, UIResult};
use crate::ui::layout_engine::DockPosition;

/// Panel manager for dock system
pub struct PanelManager {
    panels: HashMap<String, PanelInfo>,
    dock_panels: HashMap<DockPosition, Vec<String>>,
    active_panels: HashMap<DockPosition, Option<String>>,
}

/// Panel information for storage
#[derive(Debug)]
pub struct PanelInfo {
    pub name: String,
    pub title: String,
    pub icon: Option<String>,
}

/// Trait for UI panels
pub trait Panel: Send + Sync {
    /// Get the panel's unique name
    fn name(&self) -> &str;
    
    /// Get the panel's display title
    fn title(&self) -> String;
    
    /// Get the panel's icon (optional)
    fn icon(&self) -> Option<String> {
        None
    }
    
    /// Render the panel content
    fn render(&self) -> AnyElement;
    
    /// Check if panel can dock at the specified position
    fn can_dock_at(&self, position: DockPosition) -> bool {
        match position {
            DockPosition::Left | DockPosition::Right => true,
            DockPosition::Bottom => false,
        }
    }
    
    /// Called when panel is shown
    fn on_show(&mut self) {}
    
    /// Called when panel is hidden
    fn on_hide(&mut self) {}
}

impl PanelManager {
    /// Create a new panel manager
    pub fn new(_cx: &mut App) -> UIResult<Self> {
        Ok(Self {
            panels: HashMap::new(),
            dock_panels: HashMap::new(),
            active_panels: HashMap::new(),
        })
    }
    
    /// Initialize the panel manager
    pub fn initialize(&self, _cx: &mut App) -> anyhow::Result<()> {
        // TODO: Initialize default panels
        Ok(())
    }
    
    /// Register a new panel
    pub fn register_panel<P: Panel + 'static>(&mut self, panel: P) -> UIResult<()> {
        let name = panel.name().to_string();
        
        if self.panels.contains_key(&name) {
            return Err(UIError::PanelError(format!("Panel '{}' already registered", name)));
        }
        
        let panel_info = PanelInfo {
            name: name.clone(),
            title: panel.title(),
            icon: panel.icon(),
        };
        
        self.panels.insert(name, panel_info);
        Ok(())
    }
    
    /// Show a panel at the specified dock position
    pub fn show_panel(&mut self, name: &str, position: DockPosition, _cx: &mut App) -> UIResult<()> {
        if !self.panels.contains_key(name) {
            return Err(UIError::PanelError(format!("Panel '{}' not found", name)));
        }
        
        // Add panel to dock if not already there
        let dock_panels = self.dock_panels.entry(position).or_insert_with(Vec::new);
        if !dock_panels.contains(&name.to_string()) {
            dock_panels.push(name.to_string());
        }
        
        // Set as active panel for this dock
        self.active_panels.insert(position, Some(name.to_string()));
        
        // TODO: Trigger panel show event
        Ok(())
    }
    
    /// Hide a panel
    pub fn hide_panel(&mut self, name: &str, _cx: &mut App) -> UIResult<()> {
        if !self.panels.contains_key(name) {
            return Err(UIError::PanelError(format!("Panel '{}' not found", name)));
        }
        
        // Remove from active panels
        for (_position, active_panel) in self.active_panels.iter_mut() {
            if let Some(active_name) = active_panel {
                if active_name == name {
                    *active_panel = None;
                    break;
                }
            }
        }
        
        // TODO: Trigger panel hide event
        Ok(())
    }
    
    /// Toggle dock visibility
    pub fn toggle_dock(&mut self, position: DockPosition, _cx: &mut App) -> UIResult<()> {
        // TODO: Implement dock toggle
        Ok(())
    }
    
    /// Get panels in a specific dock
    pub fn get_dock_panels(&self, position: DockPosition) -> Vec<&str> {
        self.dock_panels.get(&position)
            .map(|panels| panels.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }
    
    /// Get active panel for a dock
    pub fn get_active_panel(&self, position: DockPosition) -> Option<&str> {
        self.active_panels.get(&position)
            .and_then(|panel| panel.as_deref())
    }
    
    /// Get panel info by name
    pub fn get_panel_info(&self, name: &str) -> Option<&PanelInfo> {
        self.panels.get(name)
    }
}