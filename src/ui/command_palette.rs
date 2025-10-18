//! Command Palette
//! 
//! Unified command interface with fuzzy search, similar to Zed's command palette.
//! Provides command registration, execution, and extensibility.

use std::collections::HashMap;
use gpui::{App, Entity, KeyBinding};
use crate::ui::{UIError, UIResult};

/// Command palette for unified command access
pub struct CommandPalette {
    commands: HashMap<String, Command>,
    history: CommandHistory,
    interceptors: Vec<Box<dyn CommandInterceptor>>,
}

/// Individual command definition
#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub keybinding: Option<KeyBinding>,
}

/// Command execution history
#[derive(Debug, Default)]
pub struct CommandHistory {
    recent_commands: Vec<String>,
    command_counts: HashMap<String, u32>,
}

/// Trait for command interceptors (extensibility)
pub trait CommandInterceptor: Send + Sync {
    /// Intercept command queries and provide additional commands
    fn intercept(&self, query: &str) -> Option<Vec<Command>>;
}

impl CommandPalette {
    /// Create a new command palette
    pub fn new(_cx: &mut App) -> UIResult<Self> {
        Ok(Self {
            commands: HashMap::new(),
            history: CommandHistory::default(),
            interceptors: Vec::new(),
        })
    }
    
    /// Initialize the command palette
    pub fn initialize(&self, _cx: &mut App) -> anyhow::Result<()> {
        // TODO: Register default commands
        Ok(())
    }
    
    /// Register a command
    pub fn register_command(&mut self, command: Command) -> UIResult<()> {
        let name = command.name.clone();
        
        if self.commands.contains_key(&name) {
            return Err(UIError::CommandPaletteError(format!(
                "Command '{}' already registered", name
            )));
        }
        
        self.commands.insert(name, command);
        Ok(())
    }
    
    /// Register a command interceptor
    pub fn register_interceptor<I: CommandInterceptor + 'static>(&mut self, interceptor: I) {
        self.interceptors.push(Box::new(interceptor));
    }
    
    /// Execute a command by name
    pub fn execute_command(&mut self, command_name: &str, _cx: &mut App) -> UIResult<()> {
        if let Some(command) = self.commands.get(command_name) {
            // Add to history
            self.history.recent_commands.push(command_name.to_string());
            *self.history.command_counts.entry(command_name.to_string()).or_insert(0) += 1;
            
            // Keep history size manageable
            if self.history.recent_commands.len() > 100 {
                self.history.recent_commands.remove(0);
            }
            
            // TODO: Execute the command action
            Ok(())
        } else {
            Err(UIError::CommandPaletteError(format!(
                "Command '{}' not found", command_name
            )))
        }
    }
    
    /// Search commands with fuzzy matching
    pub fn search_commands(&self, query: &str) -> Vec<&Command> {
        let mut results = Vec::new();
        
        // Search registered commands
        for command in self.commands.values() {
            if self.matches_query(&command.name, query) || 
               command.description.as_ref().map_or(false, |desc| self.matches_query(desc, query)) {
                results.push(command);
            }
        }
        
        // TODO: Add fuzzy search scoring and sorting
        // TODO: Add interceptor results
        
        results
    }
    
    /// Get command history
    pub fn get_recent_commands(&self) -> &[String] {
        &self.history.recent_commands
    }
    
    /// Get frequently used commands
    pub fn get_frequent_commands(&self) -> Vec<(&str, u32)> {
        let mut commands: Vec<_> = self.history.command_counts.iter()
            .map(|(name, count)| (name.as_str(), *count))
            .collect();
        
        commands.sort_by(|a, b| b.1.cmp(&a.1));
        commands
    }
    
    /// Simple query matching (TODO: replace with proper fuzzy search)
    fn matches_query(&self, text: &str, query: &str) -> bool {
        text.to_lowercase().contains(&query.to_lowercase())
    }
}