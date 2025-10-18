//! Layout Engine
//! 
//! Manages window layouts, pane splits, and layout persistence following
//! Zed's workspace patterns with center pane groups and surrounding docks.

use std::collections::HashMap;
use std::ops::Range;
use gpui::{App, Entity, Axis, WindowHandle, Context, EntityId, AppContext, Render, IntoElement, InteractiveElement, Styled, ParentElement, div};
use serde::{Serialize, Deserialize};
use crate::ui::{UIError, UIResult};

/// Layout engine for managing panes and docks
pub struct LayoutEngine {
    center_pane_group: Option<Entity<PaneGroup>>,
    docks: HashMap<DockPosition, Entity<Dock>>,
    layout_state: LayoutState,
    center_layout_mode: CenterLayoutMode,
    drag_state: Option<DragState>,
    transition_state: Option<LayoutTransition>,
}

/// Layout transition state for animations
#[derive(Debug, Clone)]
pub struct LayoutTransition {
    pub from_mode: CenterLayoutMode,
    pub to_mode: CenterLayoutMode,
    pub start_time: std::time::Instant,
    pub duration: std::time::Duration,
    pub progress: f32, // 0.0 to 1.0
}

/// Center layout mode for query editor and results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CenterLayoutMode {
    /// Query editor | Result panel (side by side)
    SideBySide,
    /// Query editor above, Result panel below
    TopBottom,
}

/// Dock position enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DockPosition {
    Left,
    Right,
    Bottom,
}

/// Pane group structure following Zed's pattern
#[derive(Debug, Clone)]
pub struct PaneGroup {
    pub axis: Option<Axis>,
    pub children: Vec<PaneGroupChild>,
    pub flex_basis: Option<f32>,
}

/// Pane group child types
#[derive(Debug, Clone)]
pub enum PaneGroupChild {
    Group(PaneGroup),
    Pane(Entity<Pane>),
}

impl PaneGroupChild {
    /// Get the flex basis for this child
    pub fn flex_basis(&self) -> Option<f32> {
        match self {
            PaneGroupChild::Group(group) => group.flex_basis,
            PaneGroupChild::Pane(_) => Some(1.0), // Default flex basis for panes
        }
    }
}

/// Individual pane
#[derive(Debug)]
pub struct Pane {
    pub id: String,
    pub active: bool,
    pub flex_basis: Option<f32>,
    pub min_size: f32,
    pub max_size: Option<f32>,
}

/// Drag state for pane operations
#[derive(Debug, Clone)]
pub struct DragState {
    pub dragging_pane: String,
    pub drag_position: (f32, f32),
    pub valid_drop_zones: Vec<DropZone>,
}

/// Drop zone for drag operations
#[derive(Debug, Clone)]
pub struct DropZone {
    pub id: String,
    pub bounds: (f32, f32, f32, f32), // x, y, width, height
    pub drop_type: DropType,
}

/// Type of drop operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropType {
    Split(Axis),
    Replace,
    Dock(DockPosition),
}

/// Visual feedback for drag operations
#[derive(Debug, Clone)]
pub struct DragVisualFeedback {
    pub dragging_pane: String,
    pub drag_position: (f32, f32),
    pub valid_drop_zones: Vec<DropZone>,
    pub active_drop_zone: Option<DropZone>,
    pub highlight_color: (f32, f32, f32, f32), // RGBA
}

/// Dock container with resizing and panel management
#[derive(Debug)]
pub struct Dock {
    pub position: DockPosition,
    pub visible: bool,
    pub size: f32,
    pub min_size: f32,
    pub max_size: f32,
    pub panels: Vec<String>,
    pub active_panel: Option<String>,
    pub resizing: bool,
    pub resize_handle_size: f32,
}

/// Layout state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutState {
    pub center_layout_mode: CenterLayoutMode,
    pub dock_states: HashMap<DockPosition, DockState>,
}

/// Serializable dock state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockState {
    pub visible: bool,
    pub size: f32,
    pub active_panel: Option<String>,
    pub panels: Vec<String>,
}

impl LayoutEngine {
    /// Create a new layout engine
    pub fn new(cx: &mut App) -> UIResult<Self> {
        let mut engine = Self {
            center_pane_group: None,
            docks: HashMap::new(),
            layout_state: LayoutState {
                center_layout_mode: CenterLayoutMode::SideBySide,
                dock_states: HashMap::new(),
            },
            center_layout_mode: CenterLayoutMode::SideBySide,
            drag_state: None,
            transition_state: None,
        };
        
        // Initialize default center pane group
        engine.initialize_default_layout(cx)?;
        
        Ok(engine)
    }
    
    /// Initialize the layout engine
    pub fn initialize(&self, _cx: &mut App) -> anyhow::Result<()> {
        // Layout is initialized in new()
        Ok(())
    }
    
    /// Initialize default layout with center pane group
    fn initialize_default_layout(&mut self, cx: &mut App) -> UIResult<()> {
        // Create initial center pane group based on layout mode
        let center_group = cx.new(|cx| {
            match self.center_layout_mode {
                CenterLayoutMode::SideBySide => {
                    // Create side-by-side layout: Query Editor | Results
                    let query_pane = cx.new(|_| Pane {
                        id: self.generate_pane_id(),
                        active: true,
                        flex_basis: Some(0.4), // 40% for query editor
                        min_size: 200.0,
                        max_size: None,
                    });
                    
                    let results_pane = cx.new(|_| Pane {
                        id: self.generate_pane_id(),
                        active: false,
                        flex_basis: Some(0.6), // 60% for results
                        min_size: 300.0,
                        max_size: None,
                    });
                    
                    PaneGroup {
                        axis: Some(Axis::Horizontal),
                        children: vec![
                            PaneGroupChild::Pane(query_pane),
                            PaneGroupChild::Pane(results_pane),
                        ],
                        flex_basis: Some(1.0),
                    }
                }
                CenterLayoutMode::TopBottom => {
                    // Create top-bottom layout: Query Editor above Results
                    let query_pane = cx.new(|_| Pane {
                        id: self.generate_pane_id(),
                        active: true,
                        flex_basis: Some(0.3), // 30% for query editor
                        min_size: 150.0,
                        max_size: None,
                    });
                    
                    let results_pane = cx.new(|_| Pane {
                        id: self.generate_pane_id(),
                        active: false,
                        flex_basis: Some(0.7), // 70% for results
                        min_size: 200.0,
                        max_size: None,
                    });
                    
                    PaneGroup {
                        axis: Some(Axis::Vertical),
                        children: vec![
                            PaneGroupChild::Pane(query_pane),
                            PaneGroupChild::Pane(results_pane),
                        ],
                        flex_basis: Some(1.0),
                    }
                }
            }
        });
        
        self.center_pane_group = Some(center_group);
        Ok(())
    }
    
    /// Generate unique pane ID using cuid2
    fn generate_pane_id(&mut self) -> String {
        format!("pane_{}", cuid2::create_id())
    }
    
    /// Split a pane in the specified direction
    pub fn split_pane(&mut self, direction: Axis, cx: &mut App) -> UIResult<Entity<Pane>> {
        self.split_pane_by_id(None, direction, cx)
    }
    
    /// Split a specific pane by ID
    pub fn split_pane_by_id(&mut self, pane_id: Option<&str>, direction: Axis, cx: &mut App) -> UIResult<Entity<Pane>> {
        let center_group = self.center_pane_group.as_ref()
            .ok_or_else(|| UIError::LayoutError("No center pane group".to_string()))?
            .clone();
        
        // Create new pane
        let new_pane = cx.new(|_| Pane {
            id: self.generate_pane_id(),
            active: false,
            flex_basis: Some(0.5), // Split 50/50
            min_size: 100.0,
            max_size: None,
        });
        
        // Update the center group to include the split
        center_group.update(cx, |group, cx| {
            self.split_pane_group(group, pane_id, direction, new_pane.clone(), cx)
        })?;
        
        Ok(new_pane)
    }
    
    /// Split a pane group recursively
    fn split_pane_group(
        &mut self,
        group: &mut PaneGroup,
        target_pane_id: Option<&str>,
        direction: Axis,
        new_pane: Entity<Pane>,
        cx: &mut Context<PaneGroup>,
    ) -> UIResult<()> {
        // If no target pane specified, split the first pane
        let target_id = target_pane_id.unwrap_or("pane_1");
        
        // Find and split the target pane
        for (index, child) in group.children.iter_mut().enumerate() {
            match child {
                PaneGroupChild::Pane(pane_entity) => {
                    let pane_id = pane_entity.read(cx).id.clone();
                    if pane_id == target_id {
                        // Split this pane
                        let original_pane = pane_entity.clone();
                        
                        // Update original pane flex basis
                        original_pane.update(cx, |pane, _| {
                            pane.flex_basis = Some(0.5);
                        });
                        
                        // Create new group with split panes
                        let split_group = PaneGroup {
                            axis: Some(direction),
                            children: vec![
                                PaneGroupChild::Pane(original_pane),
                                PaneGroupChild::Pane(new_pane),
                            ],
                            flex_basis: group.children[index].flex_basis(),
                        };
                        
                        // Replace the pane with the new group
                        group.children[index] = PaneGroupChild::Group(split_group);
                        return Ok(());
                    }
                }
                PaneGroupChild::Group(sub_group) => {
                    // Recursively search in sub-groups - create a separate method to avoid borrow issues
                    let found = Self::find_and_split_pane_in_group(sub_group, target_id, direction, new_pane.clone(), cx);
                    if found.is_ok() {
                        return Ok(());
                    }
                }
            }
        }
        
        Err(UIError::LayoutError(format!("Pane {} not found for splitting", target_id)))
    }
    
    /// Helper method to find and split pane in group (static to avoid borrow issues)
    fn find_and_split_pane_in_group(
        group: &mut PaneGroup,
        target_id: &str,
        direction: Axis,
        new_pane: Entity<Pane>,
        cx: &mut Context<PaneGroup>,
    ) -> UIResult<()> {
        for (index, child) in group.children.iter_mut().enumerate() {
            match child {
                PaneGroupChild::Pane(pane_entity) => {
                    let pane_id = pane_entity.read(cx).id.clone();
                    if pane_id == target_id {
                        // Split this pane
                        let original_pane = pane_entity.clone();
                        
                        // Update original pane flex basis
                        original_pane.update(cx, |pane, _| {
                            pane.flex_basis = Some(0.5);
                        });
                        
                        // Create new group with split panes
                        let split_group = PaneGroup {
                            axis: Some(direction),
                            children: vec![
                                PaneGroupChild::Pane(original_pane),
                                PaneGroupChild::Pane(new_pane),
                            ],
                            flex_basis: group.children[index].flex_basis(),
                        };
                        
                        // Replace the pane with the new group
                        group.children[index] = PaneGroupChild::Group(split_group);
                        return Ok(());
                    }
                }
                PaneGroupChild::Group(sub_group) => {
                    if let Ok(()) = Self::find_and_split_pane_in_group(sub_group, target_id, direction, new_pane.clone(), cx) {
                        return Ok(());
                    }
                }
            }
        }
        
        Err(UIError::LayoutError(format!("Pane {} not found for splitting", target_id)))
    }
    
    /// Find the first pane ID in a group
    fn find_first_pane_id(&self, group: &PaneGroup) -> Option<&str> {
        for child in &group.children {
            match child {
                PaneGroupChild::Pane(_) => {
                    // We can't access the pane ID here without context, so return a default
                    return Some("pane_1");
                }
                PaneGroupChild::Group(sub_group) => {
                    if let Some(id) = self.find_first_pane_id(sub_group) {
                        return Some(id);
                    }
                }
            }
        }
        None
    }
    
    /// Resize a pane by the specified delta
    pub fn resize_pane(&mut self, pane_id: &str, delta: f32, cx: &mut App) -> UIResult<()> {
        let center_group = self.center_pane_group.as_ref()
            .ok_or_else(|| UIError::LayoutError("No center pane group".to_string()))?
            .clone();
        
        center_group.update(cx, |group, cx| {
            self.resize_pane_in_group(group, pane_id, delta, cx)
        })?;
        
        Ok(())
    }
    
    /// Resize a pane within a group
    fn resize_pane_in_group(
        &mut self,
        group: &mut PaneGroup,
        pane_id: &str,
        delta: f32,
        cx: &mut Context<PaneGroup>,
    ) -> UIResult<()> {
        for child in &mut group.children {
            match child {
                PaneGroupChild::Pane(pane_entity) => {
                    let current_id = pane_entity.read(cx).id.clone();
                    if current_id == pane_id {
                        pane_entity.update(cx, |pane, _| {
                            if let Some(current_basis) = pane.flex_basis {
                                let new_basis = (current_basis + delta).max(0.1).min(0.9);
                                pane.flex_basis = Some(new_basis);
                            }
                        });
                        return Ok(());
                    }
                }
                PaneGroupChild::Group(sub_group) => {
                    if let Ok(()) = self.resize_pane_in_group(sub_group, pane_id, delta, cx) {
                        return Ok(());
                    }
                }
            }
        }
        
        Err(UIError::LayoutError(format!("Pane {} not found for resizing", pane_id)))
    }
    
    /// Get all panes in the layout
    pub fn get_all_panes(&self, cx: &App) -> Vec<String> {
        let mut panes = Vec::new();
        if let Some(center_group) = &self.center_pane_group {
            self.collect_panes_from_group(&center_group.read(cx), &mut panes, cx);
        }
        panes
    }
    
    /// Collect pane IDs from a group recursively
    fn collect_panes_from_group(&self, group: &PaneGroup, panes: &mut Vec<String>, cx: &App) {
        for child in &group.children {
            match child {
                PaneGroupChild::Pane(pane_entity) => {
                    panes.push(pane_entity.read(cx).id.clone());
                }
                PaneGroupChild::Group(sub_group) => {
                    self.collect_panes_from_group(sub_group, panes, cx);
                }
            }
        }
    }
    
    /// Get the center pane group
    pub fn center_pane_group(&self) -> Option<&Entity<PaneGroup>> {
        self.center_pane_group.as_ref()
    }
    
    /// Check if a pane exists
    pub fn pane_exists(&self, pane_id: &str, cx: &App) -> bool {
        self.get_all_panes(cx).contains(&pane_id.to_string())
    }
    
    /// Toggle between center layout modes
    pub fn toggle_center_layout(&mut self, cx: &mut App) -> UIResult<()> {
        let new_mode = match self.center_layout_mode {
            CenterLayoutMode::SideBySide => CenterLayoutMode::TopBottom,
            CenterLayoutMode::TopBottom => CenterLayoutMode::SideBySide,
        };
        
        self.set_center_layout(new_mode, cx)
    }
    
    /// Set specific center layout mode
    pub fn set_center_layout(&mut self, layout: CenterLayoutMode, cx: &mut App) -> UIResult<()> {
        if self.center_layout_mode == layout {
            return Ok(()); // No change needed
        }
        
        let old_mode = self.center_layout_mode;
        self.center_layout_mode = layout;
        self.layout_state.center_layout_mode = layout;
        
        // Apply layout changes with smooth transition
        self.apply_center_layout_change(old_mode, layout, cx)?;
        
        Ok(())
    }
    
    /// Apply center layout change with smooth transitions
    fn apply_center_layout_change(
        &mut self,
        old_mode: CenterLayoutMode,
        new_mode: CenterLayoutMode,
        cx: &mut App,
    ) -> UIResult<()> {
        // Start transition animation
        self.transition_state = Some(LayoutTransition {
            from_mode: old_mode,
            to_mode: new_mode,
            start_time: std::time::Instant::now(),
            duration: self.get_transition_duration(),
            progress: 0.0,
        });
        
        if let Some(center_group) = &self.center_pane_group {
            center_group.update(cx, |group, cx| {
                // Update the axis and flex basis based on new layout mode
                match new_mode {
                    CenterLayoutMode::SideBySide => {
                        group.axis = Some(Axis::Horizontal);
                        // Update pane flex basis for side-by-side layout
                        self.update_pane_flex_for_layout(group, 0.4, 0.6, cx);
                    }
                    CenterLayoutMode::TopBottom => {
                        group.axis = Some(Axis::Vertical);
                        // Update pane flex basis for top-bottom layout
                        self.update_pane_flex_for_layout(group, 0.3, 0.7, cx);
                    }
                }
            });
        }
        
        // Schedule transition completion
        self.schedule_transition_completion(cx);
        
        Ok(())
    }
    
    /// Schedule transition completion
    fn schedule_transition_completion(&mut self, _cx: &mut App) {
        // In a real implementation, this would use gpui's animation system
        // For now, we'll mark the transition as complete immediately
        // This could be enhanced with actual animation frames
        if let Some(transition) = &mut self.transition_state {
            transition.progress = 1.0;
        }
        
        // Clear transition state after completion
        self.transition_state = None;
    }
    
    /// Update pane flex basis for layout change
    fn update_pane_flex_for_layout(
        &self,
        group: &mut PaneGroup,
        first_flex: f32,
        second_flex: f32,
        cx: &mut Context<PaneGroup>,
    ) {
        if group.children.len() >= 2 {
            // Update first pane (query editor)
            if let PaneGroupChild::Pane(pane) = &group.children[0] {
                pane.update(cx, |p, _| {
                    p.flex_basis = Some(first_flex);
                });
            }
            
            // Update second pane (results)
            if let PaneGroupChild::Pane(pane) = &group.children[1] {
                pane.update(cx, |p, _| {
                    p.flex_basis = Some(second_flex);
                });
            }
        }
    }
    
    /// Get layout transition animation duration
    pub fn get_transition_duration(&self) -> std::time::Duration {
        std::time::Duration::from_millis(300) // 300ms smooth transition
    }
    
    /// Check if layout is currently transitioning
    pub fn is_transitioning(&self) -> bool {
        self.transition_state.is_some()
    }
    
    /// Get current transition progress (0.0 to 1.0)
    pub fn transition_progress(&self) -> Option<f32> {
        self.transition_state.as_ref().map(|t| t.progress)
    }
    
    /// Update transition progress (called during animation frames)
    pub fn update_transition(&mut self) -> bool {
        if let Some(transition) = &mut self.transition_state {
            let elapsed = transition.start_time.elapsed();
            transition.progress = (elapsed.as_millis() as f32 / transition.duration.as_millis() as f32).min(1.0);
            
            if transition.progress >= 1.0 {
                self.transition_state = None;
                return true; // Transition completed
            }
        }
        false
    }
    
    /// Apply easing function to transition progress
    pub fn ease_transition_progress(progress: f32) -> f32 {
        // Ease-in-out cubic function for smooth transitions
        if progress < 0.5 {
            4.0 * progress * progress * progress
        } else {
            1.0 - (-2.0 * progress + 2.0).powi(3) / 2.0
        }
    }
    
    /// Start dragging a pane
    pub fn start_pane_drag(&mut self, pane_id: String, position: (f32, f32), cx: &App) -> UIResult<()> {
        // Calculate valid drop zones
        let drop_zones = self.calculate_drop_zones(cx);
        
        self.drag_state = Some(DragState {
            dragging_pane: pane_id,
            drag_position: position,
            valid_drop_zones: drop_zones,
        });
        
        Ok(())
    }
    
    /// Update drag position
    pub fn update_pane_drag(&mut self, position: (f32, f32)) -> UIResult<()> {
        if let Some(drag_state) = &mut self.drag_state {
            drag_state.drag_position = position;
        }
        Ok(())
    }
    
    /// End pane drag operation
    pub fn end_pane_drag(&mut self, drop_position: (f32, f32), cx: &mut App) -> UIResult<Option<String>> {
        let drag_state = self.drag_state.take()
            .ok_or_else(|| UIError::LayoutError("No active drag operation".to_string()))?;
        
        // Find the drop zone at the drop position
        let drop_zone = Self::find_drop_zone_at_position_static(drop_position, &drag_state.valid_drop_zones);
        
        if let Some(zone) = drop_zone {
            let zone_id = zone.id.clone();
            self.execute_drop_operation(&drag_state.dragging_pane, &zone, cx)?;
            Ok(Some(zone_id))
        } else {
            // Invalid drop, return pane to original position
            Ok(None)
        }
    }
    
    /// Cancel drag operation
    pub fn cancel_pane_drag(&mut self) {
        self.drag_state = None;
    }
    
    /// Calculate valid drop zones for current layout
    fn calculate_drop_zones(&self, cx: &App) -> Vec<DropZone> {
        let mut zones = Vec::new();
        
        // Add split zones for existing panes
        if let Some(center_group) = &self.center_pane_group {
            self.add_split_zones_for_group(&center_group.read(cx), &mut zones, cx);
        }
        
        // Add dock zones
        zones.extend(self.calculate_dock_zones());
        
        zones
    }
    
    /// Add split zones for a pane group
    fn add_split_zones_for_group(&self, group: &PaneGroup, zones: &mut Vec<DropZone>, cx: &App) {
        for (index, child) in group.children.iter().enumerate() {
            match child {
                PaneGroupChild::Pane(pane_entity) => {
                    let pane_id = &pane_entity.read(cx).id;
                    
                    // Add horizontal split zones
                    zones.push(DropZone {
                        id: format!("split_left_{}", cuid2::create_id()),
                        bounds: (0.0, 0.0, 100.0, 100.0), // Placeholder bounds
                        drop_type: DropType::Split(Axis::Horizontal),
                    });
                    
                    zones.push(DropZone {
                        id: format!("split_right_{}", cuid2::create_id()),
                        bounds: (100.0, 0.0, 100.0, 100.0), // Placeholder bounds
                        drop_type: DropType::Split(Axis::Horizontal),
                    });
                    
                    // Add vertical split zones
                    zones.push(DropZone {
                        id: format!("split_top_{}", cuid2::create_id()),
                        bounds: (0.0, 0.0, 200.0, 50.0), // Placeholder bounds
                        drop_type: DropType::Split(Axis::Vertical),
                    });
                    
                    zones.push(DropZone {
                        id: format!("split_bottom_{}", cuid2::create_id()),
                        bounds: (0.0, 50.0, 200.0, 50.0), // Placeholder bounds
                        drop_type: DropType::Split(Axis::Vertical),
                    });
                    
                    // Add replace zone
                    zones.push(DropZone {
                        id: format!("replace_{}", cuid2::create_id()),
                        bounds: (25.0, 25.0, 150.0, 50.0), // Center area for replacement
                        drop_type: DropType::Replace,
                    });
                }
                PaneGroupChild::Group(sub_group) => {
                    self.add_split_zones_for_group(sub_group, zones, cx);
                }
            }
        }
    }
    
    /// Calculate dock drop zones
    fn calculate_dock_zones(&self) -> Vec<DropZone> {
        vec![
            DropZone {
                id: format!("dock_left_{}", cuid2::create_id()),
                bounds: (0.0, 0.0, 50.0, 400.0), // Left edge
                drop_type: DropType::Dock(DockPosition::Left),
            },
            DropZone {
                id: format!("dock_right_{}", cuid2::create_id()),
                bounds: (750.0, 0.0, 50.0, 400.0), // Right edge
                drop_type: DropType::Dock(DockPosition::Right),
            },
            DropZone {
                id: format!("dock_bottom_{}", cuid2::create_id()),
                bounds: (0.0, 350.0, 800.0, 50.0), // Bottom edge
                drop_type: DropType::Dock(DockPosition::Bottom),
            },
        ]
    }
    
    /// Find drop zone at specific position
    fn find_drop_zone_at_position<'a>(&self, position: (f32, f32), zones: &'a [DropZone]) -> Option<&'a DropZone> {
        Self::find_drop_zone_at_position_static(position, zones)
    }
    
    /// Static version of find_drop_zone_at_position to avoid lifetime issues
    fn find_drop_zone_at_position_static(position: (f32, f32), zones: &[DropZone]) -> Option<&DropZone> {
        zones.iter().find(|zone| {
            let (x, y) = position;
            let (zx, zy, zw, zh) = zone.bounds;
            x >= zx && x <= zx + zw && y >= zy && y <= zy + zh
        })
    }
    
    /// Execute drop operation
    fn execute_drop_operation(&mut self, pane_id: &str, drop_zone: &DropZone, cx: &mut App) -> UIResult<()> {
        match &drop_zone.drop_type {
            DropType::Split(axis) => {
                // Split the target pane
                self.split_pane_by_id(Some(pane_id), *axis, cx)?;
            }
            DropType::Replace => {
                // Replace operation - for now, just log it
                tracing::info!("Replace operation for pane {} in zone {}", pane_id, drop_zone.id);
            }
            DropType::Dock(position) => {
                // Move pane to dock - for now, just log it
                tracing::info!("Dock operation for pane {} to {:?}", pane_id, position);
            }
        }
        Ok(())
    }
    
    /// Get current drag state
    pub fn drag_state(&self) -> Option<&DragState> {
        self.drag_state.as_ref()
    }
    
    /// Get visual feedback for drag operation
    pub fn get_drag_visual_feedback(&self) -> Option<DragVisualFeedback> {
        self.drag_state.as_ref().map(|drag_state| {
            let active_zone = Self::find_drop_zone_at_position_static(
                drag_state.drag_position,
                &drag_state.valid_drop_zones,
            );
            
            DragVisualFeedback {
                dragging_pane: drag_state.dragging_pane.clone(),
                drag_position: drag_state.drag_position,
                valid_drop_zones: drag_state.valid_drop_zones.clone(),
                active_drop_zone: active_zone.cloned(),
                highlight_color: Self::get_drop_zone_highlight_color_static(active_zone),
            }
        })
    }
    
    /// Get highlight color for drop zone
    fn get_drop_zone_highlight_color(&self, zone: Option<&DropZone>) -> (f32, f32, f32, f32) {
        Self::get_drop_zone_highlight_color_static(zone)
    }
    
    /// Static version of get_drop_zone_highlight_color
    fn get_drop_zone_highlight_color_static(zone: Option<&DropZone>) -> (f32, f32, f32, f32) {
        match zone.map(|z| &z.drop_type) {
            Some(DropType::Split(_)) => (0.2, 0.6, 1.0, 0.3), // Blue for splits
            Some(DropType::Replace) => (1.0, 0.6, 0.2, 0.3),   // Orange for replace
            Some(DropType::Dock(_)) => (0.6, 1.0, 0.2, 0.3),   // Green for docks
            None => (0.5, 0.5, 0.5, 0.1),                      // Gray for invalid
        }
    }
    
    /// Get current center layout mode
    pub fn center_layout_mode(&self) -> CenterLayoutMode {
        self.center_layout_mode
    }
    
    /// Save current layout state
    pub fn save_layout(&self, cx: &App) -> UIResult<SerializedLayout> {
        let center_pane_group = if let Some(group_entity) = &self.center_pane_group {
            Some(self.serialize_pane_group(&group_entity.read(cx), cx))
        } else {
            None
        };
        
        Ok(SerializedLayout {
            center_layout_mode: self.center_layout_mode,
            center_pane_group,
            dock_states: self.layout_state.dock_states.clone(),
            window_bounds: None, // TODO: Get actual window bounds
            version: 1,
        })
    }
    
    /// Restore layout from saved state
    pub fn restore_layout(&mut self, layout: SerializedLayout, cx: &mut App) -> UIResult<()> {
        // Validate layout version
        if layout.version > 1 {
            return Err(UIError::LayoutError(
                format!("Unsupported layout version: {}", layout.version)
            ));
        }
        
        // Restore center layout mode
        self.center_layout_mode = layout.center_layout_mode;
        self.layout_state.center_layout_mode = layout.center_layout_mode;
        
        // Restore center pane group
        if let Some(serialized_group) = layout.center_pane_group {
            let restored_group = cx.new(|cx| {
                self.deserialize_pane_group(serialized_group, cx)
            });
            self.center_pane_group = Some(restored_group);
        }
        
        // Restore dock states
        self.layout_state.dock_states = layout.dock_states;
        
        // TODO: Restore window bounds if provided
        
        Ok(())
    }
    
    /// Serialize a pane group
    fn serialize_pane_group(&self, group: &PaneGroup, cx: &App) -> SerializedPaneGroup {
        let children = group.children.iter()
            .map(|child| self.serialize_pane_group_child(child, cx))
            .collect();
        
        SerializedPaneGroup {
            axis: group.axis,
            children,
            flex_basis: group.flex_basis,
        }
    }
    
    /// Serialize a pane group child
    fn serialize_pane_group_child(&self, child: &PaneGroupChild, cx: &App) -> SerializedPaneGroupChild {
        match child {
            PaneGroupChild::Group(group) => {
                SerializedPaneGroupChild::Group(self.serialize_pane_group(group, cx))
            }
            PaneGroupChild::Pane(pane_entity) => {
                let pane = pane_entity.read(cx);
                SerializedPaneGroupChild::Pane {
                    id: pane.id.clone(),
                    active: pane.active,
                    flex_basis: pane.flex_basis,
                    min_size: pane.min_size,
                    max_size: pane.max_size,
                }
            }
        }
    }
    
    /// Deserialize a pane group
    fn deserialize_pane_group(&mut self, serialized: SerializedPaneGroup, cx: &mut Context<PaneGroup>) -> PaneGroup {
        let children = serialized.children.into_iter()
            .map(|child| self.deserialize_pane_group_child(child, cx))
            .collect();
        
        PaneGroup {
            axis: serialized.axis,
            children,
            flex_basis: serialized.flex_basis,
        }
    }
    
    /// Deserialize a pane group child
    fn deserialize_pane_group_child(&mut self, serialized: SerializedPaneGroupChild, cx: &mut Context<PaneGroup>) -> PaneGroupChild {
        match serialized {
            SerializedPaneGroupChild::Group(group) => {
                PaneGroupChild::Group(self.deserialize_pane_group(group, cx))
            }
            SerializedPaneGroupChild::Pane { id, active, flex_basis, min_size, max_size } => {
                let id_clone = id.clone();
                let pane = cx.new(|_| Pane {
                    id,
                    active,
                    flex_basis,
                    min_size,
                    max_size,
                });
                
                // No need to update pane counter since we use cuid2 for unique IDs
                
                PaneGroupChild::Pane(pane)
            }
        }
    }
    
    /// Save layout to file
    pub fn save_layout_to_file(&self, path: &std::path::Path, cx: &App) -> UIResult<()> {
        let layout = self.save_layout(cx)?;
        let serialized = toml::to_string_pretty(&layout)
            .map_err(|e| UIError::LayoutError(format!("Failed to serialize layout: {}", e)))?;
        
        std::fs::write(path, serialized)
            .map_err(|e| UIError::LayoutError(format!("Failed to write layout file: {}", e)))?;
        
        Ok(())
    }
    
    /// Load layout from file
    pub fn load_layout_from_file(&mut self, path: &std::path::Path, cx: &mut App) -> UIResult<()> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| UIError::LayoutError(format!("Failed to read layout file: {}", e)))?;
        
        let layout: SerializedLayout = toml::from_str(&content)
            .map_err(|e| UIError::LayoutError(format!("Failed to parse layout file: {}", e)))?;
        
        self.restore_layout(layout, cx)?;
        
        Ok(())
    }
    
    /// Get default layout file path
    pub fn get_default_layout_path() -> std::path::PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("tottho").join("layout.toml")
        } else {
            std::path::PathBuf::from("layout.toml")
        }
    }
    
    /// Auto-save layout (called periodically or on changes)
    pub fn auto_save_layout(&self, cx: &App) -> UIResult<()> {
        let layout_path = Self::get_default_layout_path();
        
        // Ensure directory exists
        if let Some(parent) = layout_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| UIError::LayoutError(format!("Failed to create config directory: {}", e)))?;
        }
        
        self.save_layout_to_file(&layout_path, cx)?;
        
        tracing::debug!("Auto-saved layout to {:?}", layout_path);
        Ok(())
    }
    
    /// Auto-load layout on startup
    pub fn auto_load_layout(&mut self, cx: &mut App) -> UIResult<bool> {
        let layout_path = Self::get_default_layout_path();
        
        if layout_path.exists() {
            match self.load_layout_from_file(&layout_path, cx) {
                Ok(()) => {
                    tracing::debug!("Auto-loaded layout from {:?}", layout_path);
                    Ok(true)
                }
                Err(e) => {
                    tracing::warn!("Failed to auto-load layout: {}", e);
                    // Don't fail startup, just use default layout
                    Ok(false)
                }
            }
        } else {
            Ok(false)
        }
    }
    
    // === Dock Management Methods ===
    
    /// Create a new dock at the specified position
    pub fn create_dock(&mut self, position: DockPosition, cx: &mut App) -> UIResult<Entity<Dock>> {
        if self.docks.contains_key(&position) {
            return Err(UIError::LayoutError(
                format!("Dock already exists at position {:?}", position)
            ));
        }
        
        let dock_entity = cx.new(|_| Dock {
            position,
            visible: false,
            size: Self::get_default_dock_size(position),
            min_size: Self::get_min_dock_size(position),
            max_size: Self::get_max_dock_size(position),
            panels: Vec::new(),
            active_panel: None,
            resizing: false,
            resize_handle_size: 4.0, // 4px resize handle
        });
        
        self.docks.insert(position, dock_entity.clone());
        
        // Update layout state
        self.layout_state.dock_states.insert(position, DockState {
            visible: false,
            size: Self::get_default_dock_size(position),
            active_panel: None,
            panels: Vec::new(),
        });
        
        tracing::debug!("Created dock at position {:?}", position);
        Ok(dock_entity)
    }
    
    /// Get or create a dock at the specified position
    pub fn get_or_create_dock(&mut self, position: DockPosition, cx: &mut App) -> UIResult<Entity<Dock>> {
        if let Some(dock) = self.docks.get(&position) {
            Ok(dock.clone())
        } else {
            self.create_dock(position, cx)
        }
    }
    
    /// Show a dock
    pub fn show_dock(&mut self, position: DockPosition, cx: &mut App) -> UIResult<()> {
        let dock = self.get_or_create_dock(position, cx)?;
        
        dock.update(cx, |dock, _| {
            dock.visible = true;
        });
        
        // Update layout state
        if let Some(dock_state) = self.layout_state.dock_states.get_mut(&position) {
            dock_state.visible = true;
        }
        
        tracing::debug!("Showed dock at position {:?}", position);
        Ok(())
    }
    
    /// Hide a dock
    pub fn hide_dock(&mut self, position: DockPosition, cx: &mut App) -> UIResult<()> {
        if let Some(dock) = self.docks.get(&position) {
            dock.update(cx, |dock, _| {
                dock.visible = false;
            });
            
            // Update layout state
            if let Some(dock_state) = self.layout_state.dock_states.get_mut(&position) {
                dock_state.visible = false;
            }
            
            tracing::debug!("Hid dock at position {:?}", position);
        }
        Ok(())
    }
    
    /// Toggle dock visibility
    pub fn toggle_dock_visibility(&mut self, position: DockPosition, cx: &mut App) -> UIResult<bool> {
        let dock = self.get_or_create_dock(position, cx)?;
        let is_visible = dock.read(cx).visible;
        
        if is_visible {
            self.hide_dock(position, cx)?;
            Ok(false)
        } else {
            self.show_dock(position, cx)?;
            Ok(true)
        }
    }
    
    /// Resize a dock
    pub fn resize_dock(&mut self, position: DockPosition, new_size: f32, cx: &mut App) -> UIResult<()> {
        if let Some(dock) = self.docks.get(&position) {
            dock.update(cx, |dock, _| {
                let clamped_size = new_size.max(dock.min_size).min(dock.max_size);
                dock.size = clamped_size;
            });
            
            // Update layout state
            if let Some(dock_state) = self.layout_state.dock_states.get_mut(&position) {
                dock_state.size = new_size.max(Self::get_min_dock_size(position))
                    .min(Self::get_max_dock_size(position));
            }
            
            tracing::debug!("Resized dock {:?} to size {}", position, new_size);
        }
        Ok(())
    }
    
    /// Start resizing a dock
    pub fn start_dock_resize(&mut self, position: DockPosition, cx: &mut App) -> UIResult<()> {
        if let Some(dock) = self.docks.get(&position) {
            dock.update(cx, |dock, _| {
                dock.resizing = true;
            });
            tracing::debug!("Started resizing dock {:?}", position);
        }
        Ok(())
    }
    
    /// End resizing a dock
    pub fn end_dock_resize(&mut self, position: DockPosition, cx: &mut App) -> UIResult<()> {
        if let Some(dock) = self.docks.get(&position) {
            dock.update(cx, |dock, _| {
                dock.resizing = false;
            });
            tracing::debug!("Ended resizing dock {:?}", position);
        }
        Ok(())
    }
    
    /// Add a panel to a dock
    pub fn add_panel_to_dock(&mut self, position: DockPosition, panel_name: String, cx: &mut App) -> UIResult<()> {
        let dock = self.get_or_create_dock(position, cx)?;
        
        dock.update(cx, |dock, _| {
            if !dock.panels.contains(&panel_name) {
                dock.panels.push(panel_name.clone());
                
                // If no active panel, make this one active
                if dock.active_panel.is_none() {
                    dock.active_panel = Some(panel_name.clone());
                }
            }
        });
        
        // Update layout state
        if let Some(dock_state) = self.layout_state.dock_states.get_mut(&position) {
            if !dock_state.panels.contains(&panel_name) {
                dock_state.panels.push(panel_name.clone());
                
                if dock_state.active_panel.is_none() {
                    dock_state.active_panel = Some(panel_name.clone());
                }
            }
        }
        
        tracing::debug!("Added panel '{}' to dock {:?}", panel_name, position);
        Ok(())
    }
    
    /// Remove a panel from a dock
    pub fn remove_panel_from_dock(&mut self, position: DockPosition, panel_name: &str, cx: &mut App) -> UIResult<()> {
        if let Some(dock) = self.docks.get(&position) {
            dock.update(cx, |dock, _| {
                dock.panels.retain(|p| p != panel_name);
                
                // If this was the active panel, activate another one
                if dock.active_panel.as_deref() == Some(panel_name) {
                    dock.active_panel = dock.panels.first().cloned();
                }
            });
            
            // Update layout state
            if let Some(dock_state) = self.layout_state.dock_states.get_mut(&position) {
                dock_state.panels.retain(|p| p != panel_name);
                
                if dock_state.active_panel.as_deref() == Some(panel_name) {
                    dock_state.active_panel = dock_state.panels.first().cloned();
                }
            }
            
            tracing::debug!("Removed panel '{}' from dock {:?}", panel_name, position);
        }
        Ok(())
    }
    
    /// Set active panel in a dock
    pub fn set_active_panel_in_dock(&mut self, position: DockPosition, panel_name: String, cx: &mut App) -> UIResult<()> {
        if let Some(dock) = self.docks.get(&position) {
            let panel_exists = dock.read(cx).panels.contains(&panel_name);
            
            if !panel_exists {
                return Err(UIError::PanelError(
                    format!("Panel '{}' not found in dock {:?}", panel_name, position)
                ));
            }
            
            dock.update(cx, |dock, _| {
                dock.active_panel = Some(panel_name.clone());
            });
            
            // Update layout state
            if let Some(dock_state) = self.layout_state.dock_states.get_mut(&position) {
                dock_state.active_panel = Some(panel_name.clone());
            }
            
            tracing::debug!("Set active panel '{}' in dock {:?}", panel_name, position);
        }
        Ok(())
    }
    
    /// Get dock entity
    pub fn get_dock(&self, position: DockPosition) -> Option<&Entity<Dock>> {
        self.docks.get(&position)
    }
    
    /// Check if dock is visible
    pub fn is_dock_visible(&self, position: DockPosition, cx: &App) -> bool {
        self.docks.get(&position)
            .map(|dock| dock.read(cx).visible)
            .unwrap_or(false)
    }
    
    /// Get dock size
    pub fn get_dock_size(&self, position: DockPosition, cx: &App) -> f32 {
        self.docks.get(&position)
            .map(|dock| dock.read(cx).size)
            .unwrap_or(Self::get_default_dock_size(position))
    }
    
    /// Get all docks
    pub fn get_all_docks(&self) -> &HashMap<DockPosition, Entity<Dock>> {
        &self.docks
    }
    
    /// Get default dock size based on position
    fn get_default_dock_size(position: DockPosition) -> f32 {
        match position {
            DockPosition::Left | DockPosition::Right => 300.0,
            DockPosition::Bottom => 200.0,
        }
    }
    
    /// Get minimum dock size based on position
    fn get_min_dock_size(position: DockPosition) -> f32 {
        match position {
            DockPosition::Left | DockPosition::Right => 150.0,
            DockPosition::Bottom => 100.0,
        }
    }
    
    /// Get maximum dock size based on position
    fn get_max_dock_size(position: DockPosition) -> f32 {
        match position {
            DockPosition::Left | DockPosition::Right => 600.0,
            DockPosition::Bottom => 400.0,
        }
    }
    
    /// Calculate resize handle bounds for a dock
    pub fn get_dock_resize_handle_bounds(&self, position: DockPosition, dock_bounds: (f32, f32, f32, f32), cx: &App) -> Option<(f32, f32, f32, f32)> {
        let dock = self.docks.get(&position)?;
        let dock_data = dock.read(cx);
        
        if !dock_data.visible {
            return None;
        }
        
        let (x, y, width, height) = dock_bounds;
        let handle_size = dock_data.resize_handle_size;
        
        let handle_bounds = match position {
            DockPosition::Left => (x + width - handle_size / 2.0, y, handle_size, height),
            DockPosition::Right => (x - handle_size / 2.0, y, handle_size, height),
            DockPosition::Bottom => (x, y - handle_size / 2.0, width, handle_size),
        };
        
        Some(handle_bounds)
    }
    
    /// Check if a point is within a dock's resize handle
    pub fn is_point_in_dock_resize_handle(&self, position: DockPosition, point: (f32, f32), dock_bounds: (f32, f32, f32, f32), cx: &App) -> bool {
        if let Some(handle_bounds) = self.get_dock_resize_handle_bounds(position, dock_bounds, cx) {
            let (px, py) = point;
            let (hx, hy, hw, hh) = handle_bounds;
            px >= hx && px <= hx + hw && py >= hy && py <= hy + hh
        } else {
            false
        }
    }
}

impl Render for Dock {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().id("dock-hidden").size_full();
        }
        
        let dock_class = match self.position {
            DockPosition::Left => "dock-left",
            DockPosition::Right => "dock-right", 
            DockPosition::Bottom => "dock-bottom",
        };
        
        div()
            .id(dock_class)
            .size_full()
            .child(
                div()
                    .id("dock-content")
                    .size_full()
                    .child(self.render_dock_content(cx))
            )
            .child(self.render_resize_handle(cx))
    }
}

impl Dock {
    /// Render dock content with panels
    fn render_dock_content(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("dock-panels")
            .size_full()
            .child(
                div()
                    .id("dock-panel-tabs")
                    .child(self.render_panel_tabs())
            )
            .child(
                div()
                    .id("dock-panel-content")
                    .size_full()
                    .child(self.render_active_panel_content())
            )
    }
    
    /// Render panel tabs
    fn render_panel_tabs(&self) -> impl IntoElement {
        div()
            .id("panel-tabs")
            .children(
                self.panels.iter().enumerate().map(|(index, panel_name)| {
                    let is_active = self.active_panel.as_ref() == Some(panel_name);
                    let tab_class = if is_active { "panel-tab-active" } else { "panel-tab" };
                    
                    div()
                        .id(("panel-tab", index))
                        .child(panel_name.clone())
                })
            )
    }
    
    /// Render active panel content
    fn render_active_panel_content(&self) -> impl IntoElement {
        if let Some(active_panel) = &self.active_panel {
            div()
                .id("active-panel")
                .size_full()
                .child(format!("Panel: {}", active_panel))
        } else {
            div()
                .id("no-active-panel")
                .size_full()
                .child("No active panel")
        }
    }
    
    /// Render resize handle
    fn render_resize_handle(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let handle_class = match self.position {
            DockPosition::Left => "resize-handle-right",
            DockPosition::Right => "resize-handle-left",
            DockPosition::Bottom => "resize-handle-top",
        };
        
        div()
            .id(handle_class)
            .child("") // Empty content, just for the resize handle
    }
}

/// Serialized layout for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedLayout {
    pub center_layout_mode: CenterLayoutMode,
    pub center_pane_group: Option<SerializedPaneGroup>,
    pub dock_states: HashMap<DockPosition, DockState>,
    pub window_bounds: Option<WindowBounds>,
    pub version: u32, // For future compatibility
}

/// Serialized pane group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedPaneGroup {
    pub axis: Option<Axis>,
    pub children: Vec<SerializedPaneGroupChild>,
    pub flex_basis: Option<f32>,
}

/// Serialized pane group child
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializedPaneGroupChild {
    Group(SerializedPaneGroup),
    Pane {
        id: String,
        active: bool,
        flex_basis: Option<f32>,
        min_size: f32,
        max_size: Option<f32>,
    },
}

/// Window bounds for layout persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}