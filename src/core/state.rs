//! Application state management
//! 
//! Handles persistence and synchronization of application state across sessions.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use crate::core::{CoreError, Result};

/// Unique identifier for connections
pub type ConnectionId = String;

/// Workspace layout information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    /// Layout identifier
    pub id: String,
    /// Window dimensions and position
    pub window_bounds: WindowBounds,
    /// Panel states and positions
    pub panel_states: HashMap<String, PanelState>,
    /// Active pane configuration
    pub active_panes: Vec<PaneInfo>,
}

/// Window bounds information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

/// Panel state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelState {
    pub visible: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub position: PanelPosition,
}

/// Panel position enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PanelPosition {
    Left,
    Right,
    Bottom,
    Top,
}

/// Pane information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneInfo {
    pub id: String,
    pub pane_type: String,
    pub active: bool,
    pub data: serde_json::Value,
}

/// User preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Theme selection
    pub theme: String,
    /// Font settings
    pub font_family: String,
    pub font_size: u32,
    /// Editor preferences
    pub tab_size: u32,
    pub word_wrap: bool,
    /// UI preferences
    pub show_line_numbers: bool,
    pub show_minimap: bool,
    /// Query preferences
    pub auto_complete: bool,
    pub query_timeout: u32,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            font_family: "monospace".to_string(),
            font_size: 14,
            tab_size: 4,
            word_wrap: false,
            show_line_numbers: true,
            show_minimap: true,
            auto_complete: true,
            query_timeout: 30,
        }
    }
}

/// Session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// Last active workspace
    pub last_workspace: Option<String>,
    /// Recent connections
    pub recent_connections: Vec<ConnectionId>,
    /// Open tabs and editors
    pub open_editors: Vec<EditorState>,
    /// Query history
    pub query_history: Vec<QueryHistoryEntry>,
}

/// Editor state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorState {
    pub id: String,
    pub editor_type: String,
    pub connection_id: Option<ConnectionId>,
    pub content: String,
    pub cursor_position: CursorPosition,
}

/// Cursor position in editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
}

/// Query history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHistoryEntry {
    pub id: String,
    pub query: String,
    pub connection_id: ConnectionId,
    pub executed_at: chrono::DateTime<chrono::Utc>,
    pub execution_time: Option<u64>, // milliseconds
    pub success: bool,
    pub error_message: Option<String>,
}

/// Connection profile for database connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub id: ConnectionId,
    pub name: String,
    pub database_type: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    // Note: Password is not stored here - handled by credential manager
    pub ssl_mode: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
}

/// Main application state container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationState {
    /// Workspace layouts
    pub workspace_layouts: HashMap<String, WorkspaceLayout>,
    /// User preferences
    pub user_preferences: UserPreferences,
    /// Session data
    pub session_data: SessionData,
    /// Connection profiles
    pub connection_profiles: Vec<ConnectionProfile>,
    /// Application version when state was saved
    pub version: String,
}

impl ApplicationState {
    /// Create a new application state with defaults
    pub fn new() -> Self {
        Self {
            workspace_layouts: HashMap::new(),
            user_preferences: UserPreferences::default(),
            session_data: SessionData {
                last_workspace: None,
                recent_connections: Vec::new(),
                open_editors: Vec::new(),
                query_history: Vec::new(),
            },
            connection_profiles: Vec::new(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Load application state from storage
    pub async fn load() -> Result<Self> {
        // TODO: Implement actual state loading from file system
        tracing::debug!("Loading application state (placeholder)");
        Ok(Self::new())
    }

    /// Save application state to storage
    pub async fn save(&self) -> Result<()> {
        // TODO: Implement actual state saving to file system
        tracing::debug!("Saving application state (placeholder)");
        Ok(())
    }

    /// Restore session state
    pub async fn restore_session(&self) -> Result<()> {
        // TODO: Implement session restoration logic
        tracing::debug!("Restoring session state (placeholder)");
        Ok(())
    }

    /// Handle state corruption by falling back to defaults
    pub fn handle_corruption(&self) -> Result<Self> {
        tracing::warn!("State corruption detected, falling back to defaults");
        Ok(Self::new())
    }

    /// Add a connection profile
    pub fn add_connection_profile(&mut self, profile: ConnectionProfile) {
        self.connection_profiles.push(profile);
    }

    /// Remove a connection profile
    pub fn remove_connection_profile(&mut self, id: ConnectionId) -> bool {
        if let Some(pos) = self.connection_profiles.iter().position(|p| p.id == id) {
            self.connection_profiles.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get connection profile by ID
    pub fn get_connection_profile(&self, id: ConnectionId) -> Option<&ConnectionProfile> {
        self.connection_profiles.iter().find(|p| p.id == id)
    }

    /// Add query to history
    pub fn add_query_history(&mut self, entry: QueryHistoryEntry) {
        self.session_data.query_history.push(entry);
        
        // Keep only the last 1000 queries
        if self.session_data.query_history.len() > 1000 {
            self.session_data.query_history.remove(0);
        }
    }

    /// Get recent queries for a connection
    pub fn get_recent_queries(&self, connection_id: ConnectionId, limit: usize) -> Vec<&QueryHistoryEntry> {
        self.session_data
            .query_history
            .iter()
            .filter(|entry| entry.connection_id == connection_id)
            .rev()
            .take(limit)
            .collect()
    }

    /// Update workspace layout
    pub fn update_workspace_layout(&mut self, layout: WorkspaceLayout) {
        self.workspace_layouts.insert(layout.id.clone(), layout);
    }

    /// Get workspace layout
    pub fn get_workspace_layout(&self, id: &str) -> Option<&WorkspaceLayout> {
        self.workspace_layouts.get(id)
    }
}

impl Default for ApplicationState {
    fn default() -> Self {
        Self::new()
    }
}