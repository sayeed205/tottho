//! Keyboard Navigator
//! 
//! Provides keyboard-first navigation throughout the UI, including focus management,
//! tab order, and keyboard shortcuts.

use std::collections::HashMap;
use gpui::{App, FocusHandle, KeyBinding};
use crate::ui::{UIError, UIResult};

/// Keyboard navigation system
pub struct KeyboardNavigator {
    focus_handles: Vec<FocusHandle>,
    current_focus: Option<usize>,
    shortcuts: HashMap<String, KeyBinding>,
}

impl KeyboardNavigator {
    /// Create a new keyboard navigator
    pub fn new(_cx: &mut App) -> UIResult<Self> {
        Ok(Self {
            focus_handles: Vec::new(),
            current_focus: None,
            shortcuts: HashMap::new(),
        })
    }
    
    /// Initialize the keyboard navigator
    pub fn initialize(&self, _cx: &mut App) -> anyhow::Result<()> {
        // TODO: Set up default keyboard shortcuts
        Ok(())
    }
    
    /// Register a focus handle for tab navigation
    pub fn register_focus_handle(&mut self, handle: FocusHandle) {
        self.focus_handles.push(handle);
    }
    
    /// Move focus to the next element
    pub fn focus_next(&mut self, _cx: &mut App) -> UIResult<()> {
        if self.focus_handles.is_empty() {
            return Ok(());
        }
        
        let next_index = match self.current_focus {
            Some(current) => (current + 1) % self.focus_handles.len(),
            None => 0,
        };
        
        self.current_focus = Some(next_index);
        
        // TODO: Actually focus the element
        Ok(())
    }
    
    /// Move focus to the previous element
    pub fn focus_previous(&mut self, _cx: &mut App) -> UIResult<()> {
        if self.focus_handles.is_empty() {
            return Ok(());
        }
        
        let prev_index = match self.current_focus {
            Some(current) => {
                if current == 0 {
                    self.focus_handles.len() - 1
                } else {
                    current - 1
                }
            }
            None => self.focus_handles.len() - 1,
        };
        
        self.current_focus = Some(prev_index);
        
        // TODO: Actually focus the element
        Ok(())
    }
    
    /// Register a keyboard shortcut
    pub fn register_shortcut(&mut self, name: String, binding: KeyBinding) {
        self.shortcuts.insert(name, binding);
    }
    
    /// Get registered shortcuts
    pub fn get_shortcuts(&self) -> &HashMap<String, KeyBinding> {
        &self.shortcuts
    }
    
    /// Clear all focus handles (useful for cleanup)
    pub fn clear_focus_handles(&mut self) {
        self.focus_handles.clear();
        self.current_focus = None;
    }
}