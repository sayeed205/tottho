//! Panel Manager
//! 
//! Manages panels and dock system, providing registration, lifecycle management,
//! and dock operations for UI panels following Zed's dock architecture.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use gpui::{
    App, Context, FocusHandle, IntoElement, AnyElement, Entity, AppContext, Render, 
    div, InteractiveElement, Styled, Window, ParentElement
};
use theme::Theme;
use crate::ui::{UIError, UIResult, ThemeAware};
use crate::ui::layout_engine::DockPosition;

/// Panel manager for dock system
pub struct PanelManager {
    panel_entities: HashMap<String, Entity<PanelWrapper>>,
    dock_panels: HashMap<DockPosition, Vec<String>>,
    active_panels: HashMap<DockPosition, Option<String>>,
    panel_states: HashMap<String, PanelState>,
}

/// Panel wrapper for Entity management
pub struct PanelWrapper {
    panel: Box<dyn Panel>,
    focus_handle: FocusHandle,
}

/// Panel state tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelState {
    Hidden,
    Visible,
    Detached,
    Failed(String),
}

/// Panel information for storage and display
#[derive(Debug, Clone)]
pub struct PanelInfo {
    pub name: String,
    pub title: String,
    pub icon: Option<IconName>,
    pub position: DockPosition,
    pub state: PanelState,
    pub can_detach: bool,
}

/// Icon name type for panels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconName {
    Database,
    Table,
    Query,
    History,
    Settings,
    Connection,
    Inspector,
    Custom(String),
}

/// Trait for UI panels following Zed's panel patterns
pub trait Panel: Send + Sync {
    /// Get the panel's unique name (used as identifier)
    fn name(&self) -> &str;
    
    /// Get the panel's display title
    fn title(&self) -> String;
    
    /// Get the panel's icon (optional)
    fn icon(&self) -> Option<IconName> {
        None
    }
    
    /// Render the panel content with proper context
    fn render(&mut self, cx: &mut Context<PanelWrapper>) -> AnyElement;
    
    /// Check if panel can dock at the specified position
    fn can_dock_at(&self, position: DockPosition) -> bool {
        match position {
            DockPosition::Left | DockPosition::Right => true,
            DockPosition::Bottom => true, // Allow bottom docking by default
        }
    }
    
    /// Check if panel supports detachment
    fn can_detach(&self) -> bool {
        true // Most panels can be detached by default
    }
    
    /// Get preferred dock position
    fn preferred_dock_position(&self) -> DockPosition {
        DockPosition::Left
    }
    
    /// Called when panel is shown
    fn on_show(&mut self, cx: &mut Context<PanelWrapper>) {
        // Default implementation - panels can override
    }
    
    /// Called when panel is hidden
    fn on_hide(&mut self, cx: &mut Context<PanelWrapper>) {
        // Default implementation - panels can override
    }
    
    /// Called when panel is detached to separate window
    fn on_detach(&mut self, cx: &mut Context<PanelWrapper>) {
        // Default implementation - panels can override
    }
    
    /// Called when panel is reattached from separate window
    fn on_reattach(&mut self, cx: &mut Context<PanelWrapper>) {
        // Default implementation - panels can override
    }
    
    /// Get panel's minimum size
    fn min_size(&self) -> (f32, f32) {
        (200.0, 100.0) // Default minimum size
    }
    
    /// Get panel's preferred size
    fn preferred_size(&self) -> (f32, f32) {
        (300.0, 400.0) // Default preferred size
    }
}

impl PanelWrapper {
    /// Create a new panel wrapper
    pub fn new(panel: Box<dyn Panel>, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        Self {
            panel,
            focus_handle,
        }
    }
    
    /// Get the wrapped panel
    pub fn panel(&self) -> &dyn Panel {
        self.panel.as_ref()
    }
    
    /// Get mutable reference to the wrapped panel
    pub fn panel_mut(&mut self) -> &mut dyn Panel {
        self.panel.as_mut()
    }
    
    /// Get the focus handle
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }
}

impl Render for PanelWrapper {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let panel_content = self.panel.render(cx);
        div()
            .id("panel")
            .size_full()
            .child(panel_content)
    }
}

impl PanelManager {
    /// Create a new panel manager
    pub fn new(_cx: &mut App) -> UIResult<Self> {
        Ok(Self {
            panel_entities: HashMap::new(),
            dock_panels: HashMap::new(),
            active_panels: HashMap::new(),
            panel_states: HashMap::new(),
        })
    }
    
    /// Initialize the panel manager
    pub fn initialize(&mut self, cx: &mut App) -> anyhow::Result<()> {
        // Initialize dock positions with empty panel lists
        self.dock_panels.insert(DockPosition::Left, Vec::new());
        self.dock_panels.insert(DockPosition::Right, Vec::new());
        self.dock_panels.insert(DockPosition::Bottom, Vec::new());
        
        // Initialize active panels as None
        self.active_panels.insert(DockPosition::Left, None);
        self.active_panels.insert(DockPosition::Right, None);
        self.active_panels.insert(DockPosition::Bottom, None);
        
        tracing::debug!("Panel manager initialized with dock positions");
        Ok(())
    }
    
    /// Register a new panel with lifecycle management
    pub fn register_panel<P: Panel + 'static>(&mut self, panel: P, cx: &mut App) -> UIResult<()> {
        let name = panel.name().to_string();
        
        if self.panel_entities.contains_key(&name) {
            return Err(UIError::PanelError(format!("Panel '{}' already registered", name)));
        }
        
        // Create panel entity for proper lifecycle management
        let panel_entity = cx.new(|cx| {
            PanelWrapper::new(Box::new(panel), cx)
        });
        
        // Store panel entity and initial state
        self.panel_entities.insert(name.clone(), panel_entity);
        self.panel_states.insert(name.clone(), PanelState::Hidden);
        
        tracing::debug!("Registered panel: {}", name);
        Ok(())
    }
    
    /// Show a panel at the specified dock position
    pub fn show_panel(&mut self, name: &str, position: DockPosition, cx: &mut App) -> UIResult<()> {
        let panel_entity = self.panel_entities.get(name)
            .ok_or_else(|| UIError::PanelError(format!("Panel '{}' not found", name)))?
            .clone();
        
        // Check if panel can dock at this position
        let can_dock = panel_entity.read(cx).panel().can_dock_at(position);
        if !can_dock {
            return Err(UIError::PanelError(
                format!("Panel '{}' cannot dock at position {:?}", name, position)
            ));
        }
        
        // Add panel to dock if not already there
        let dock_panels = self.dock_panels.entry(position).or_insert_with(Vec::new);
        if !dock_panels.contains(&name.to_string()) {
            dock_panels.push(name.to_string());
        }
        
        // Set as active panel for this dock
        self.active_panels.insert(position, Some(name.to_string()));
        
        // Update panel state
        self.panel_states.insert(name.to_string(), PanelState::Visible);
        
        // Trigger panel show event
        panel_entity.update(cx, |wrapper, cx| {
            wrapper.panel_mut().on_show(cx);
        });
        
        tracing::debug!("Showed panel '{}' at position {:?}", name, position);
        Ok(())
    }
    
    /// Hide a panel
    pub fn hide_panel(&mut self, name: &str, cx: &mut App) -> UIResult<()> {
        let panel_entity = self.panel_entities.get(name)
            .ok_or_else(|| UIError::PanelError(format!("Panel '{}' not found", name)))?
            .clone();
        
        // Remove from active panels
        for (position, active_panel) in self.active_panels.iter_mut() {
            if let Some(active_name) = active_panel {
                if active_name == name {
                    *active_panel = None;
                    
                    // If there are other panels in this dock, activate the first one
                    if let Some(dock_panels) = self.dock_panels.get(position) {
                        if let Some(next_panel) = dock_panels.iter().find(|&p| p != name) {
                            *active_panel = Some(next_panel.clone());
                        }
                    }
                    break;
                }
            }
        }
        
        // Update panel state
        self.panel_states.insert(name.to_string(), PanelState::Hidden);
        
        // Trigger panel hide event
        panel_entity.update(cx, |wrapper, cx| {
            wrapper.panel_mut().on_hide(cx);
        });
        
        tracing::debug!("Hid panel '{}'", name);
        Ok(())
    }
    
    /// Toggle panel visibility
    pub fn toggle_panel(&mut self, name: &str, cx: &mut App) -> UIResult<()> {
        let current_state = self.panel_states.get(name)
            .ok_or_else(|| UIError::PanelError(format!("Panel '{}' not found", name)))?
            .clone();
        
        match current_state {
            PanelState::Hidden => {
                // Show panel at its preferred position
                let panel_entity = self.panel_entities.get(name).unwrap();
                let preferred_position = panel_entity.read(cx).panel().preferred_dock_position();
                self.show_panel(name, preferred_position, cx)?;
            }
            PanelState::Visible => {
                self.hide_panel(name, cx)?;
            }
            PanelState::Detached => {
                // TODO: Reattach detached panel
                tracing::warn!("Cannot toggle detached panel '{}' - reattachment not yet implemented", name);
            }
            PanelState::Failed(_) => {
                return Err(UIError::PanelError(format!("Panel '{}' is in failed state", name)));
            }
        }
        
        Ok(())
    }
    
    /// Toggle dock visibility (show/hide all panels in dock)
    pub fn toggle_dock(&mut self, position: DockPosition, cx: &mut App) -> UIResult<()> {
        let dock_panels = self.dock_panels.get(&position)
            .ok_or_else(|| UIError::LayoutError(format!("Dock position {:?} not found", position)))?
            .clone();
        
        if dock_panels.is_empty() {
            return Ok(()); // Nothing to toggle
        }
        
        // Check if dock has any visible panels
        let has_visible_panels = dock_panels.iter()
            .any(|name| matches!(self.panel_states.get(name), Some(PanelState::Visible)));
        
        if has_visible_panels {
            // Hide all panels in dock
            for panel_name in &dock_panels {
                if matches!(self.panel_states.get(panel_name), Some(PanelState::Visible)) {
                    self.hide_panel(panel_name, cx)?;
                }
            }
            self.active_panels.insert(position, None);
        } else {
            // Show the first panel in dock (or previously active one)
            if let Some(panel_name) = dock_panels.first() {
                self.show_panel(panel_name, position, cx)?;
            }
        }
        
        tracing::debug!("Toggled dock {:?}", position);
        Ok(())
    }
    
    /// Detach a panel to a separate window
    pub fn detach_panel(&mut self, name: &str, cx: &mut App) -> UIResult<()> {
        let panel_entity = self.panel_entities.get(name)
            .ok_or_else(|| UIError::PanelError(format!("Panel '{}' not found", name)))?
            .clone();
        
        // Check if panel can be detached
        let can_detach = panel_entity.read(cx).panel().can_detach();
        if !can_detach {
            return Err(UIError::PanelError(
                format!("Panel '{}' cannot be detached", name)
            ));
        }
        
        // Remove from current dock
        for dock_panels in self.dock_panels.values_mut() {
            dock_panels.retain(|p| p != name);
        }
        
        // Remove from active panels
        for active_panel in self.active_panels.values_mut() {
            if let Some(active_name) = active_panel {
                if active_name == name {
                    *active_panel = None;
                }
            }
        }
        
        // Update panel state
        self.panel_states.insert(name.to_string(), PanelState::Detached);
        
        // Trigger panel detach event
        panel_entity.update(cx, |wrapper, cx| {
            wrapper.panel_mut().on_detach(cx);
        });
        
        // TODO: Create separate window for detached panel
        tracing::info!("Detached panel '{}' (separate window creation not yet implemented)", name);
        Ok(())
    }
    
    /// Reattach a detached panel
    pub fn reattach_panel(&mut self, name: &str, position: DockPosition, cx: &mut App) -> UIResult<()> {
        let panel_entity = self.panel_entities.get(name)
            .ok_or_else(|| UIError::PanelError(format!("Panel '{}' not found", name)))?
            .clone();
        
        // Check current state
        let current_state = self.panel_states.get(name)
            .ok_or_else(|| UIError::PanelError(format!("Panel '{}' state not found", name)))?;
        
        if !matches!(current_state, PanelState::Detached) {
            return Err(UIError::PanelError(
                format!("Panel '{}' is not detached", name)
            ));
        }
        
        // Trigger panel reattach event
        panel_entity.update(cx, |wrapper, cx| {
            wrapper.panel_mut().on_reattach(cx);
        });
        
        // Show panel at specified position
        self.show_panel(name, position, cx)?;
        
        tracing::info!("Reattached panel '{}' to position {:?}", name, position);
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
    pub fn get_panel_info(&self, name: &str) -> Option<PanelInfo> {
        let panel_entity = self.panel_entities.get(name)?;
        let state = self.panel_states.get(name)?.clone();
        
        // We need to access the panel through a context, but for now return basic info
        Some(PanelInfo {
            name: name.to_string(),
            title: name.to_string(), // TODO: Get actual title from panel
            icon: None, // TODO: Get actual icon from panel
            position: DockPosition::Left, // TODO: Get actual position
            state,
            can_detach: true, // TODO: Get actual can_detach from panel
        })
    }
    
    /// Get all registered panels
    pub fn get_all_panels(&self) -> Vec<String> {
        self.panel_entities.keys().cloned().collect()
    }
    
    /// Get panel state
    pub fn get_panel_state(&self, name: &str) -> Option<&PanelState> {
        self.panel_states.get(name)
    }
    
    /// Check if panel is visible
    pub fn is_panel_visible(&self, name: &str) -> bool {
        matches!(self.panel_states.get(name), Some(PanelState::Visible))
    }
    
    /// Check if dock has visible panels
    pub fn is_dock_visible(&self, position: DockPosition) -> bool {
        self.dock_panels.get(&position)
            .map(|panels| {
                panels.iter().any(|name| self.is_panel_visible(name))
            })
            .unwrap_or(false)
    }
    
    /// Get panel entity for direct access
    pub fn get_panel_entity(&self, name: &str) -> Option<&Entity<PanelWrapper>> {
        self.panel_entities.get(name)
    }
    
    /// Set panel state (for error handling)
    pub fn set_panel_state(&mut self, name: &str, state: PanelState) -> UIResult<()> {
        if !self.panel_entities.contains_key(name) {
            return Err(UIError::PanelError(format!("Panel '{}' not found", name)));
        }
        
        self.panel_states.insert(name.to_string(), state);
        Ok(())
    }
}

impl ThemeAware for PanelManager {
    fn apply_theme(&mut self, theme: &Theme, cx: &mut Context<Self>) {
        // Notify all registered panels about theme change
        for (panel_name, panel_entity) in &self.panel_entities {
            panel_entity.update(cx, |panel_wrapper, cx| {
                // If the panel implements ThemeAware, it would handle theme changes
                // For now, we just trigger a re-render
                cx.notify();
            });
        }
        
        tracing::info!("Applied theme '{}' to all panels", theme.name);
    }
    
    fn component_name(&self) -> &'static str {
        "PanelManager"
    }
}