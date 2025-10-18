//! Application state management
//! 
//! Handles persistence and synchronization of application state across sessions.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

/// Session restoration context for coordinating restoration across components
pub struct SessionRestorationContext {
    /// Window manager for restoring window bounds
    pub window_manager: Option<Box<dyn WindowManager>>,
    /// Panel manager for restoring panel states
    pub panel_manager: Option<Box<dyn PanelManager>>,
    /// Editor manager for restoring editor states
    pub editor_manager: Option<Box<dyn EditorManager>>,
    /// Connection manager for preparing connections
    pub connection_manager: Option<Box<dyn ConnectionManager>>,
}

impl SessionRestorationContext {
    pub fn new() -> Self {
        Self {
            window_manager: None,
            panel_manager: None,
            editor_manager: None,
            connection_manager: None,
        }
    }

    pub fn set_window_bounds(&mut self, bounds: &WindowBounds) -> Result<()> {
        if let Some(ref mut manager) = self.window_manager {
            manager.set_window_bounds(bounds)
        } else {
            tracing::warn!("No window manager available for restoring window bounds");
            Ok(())
        }
    }

    pub fn set_panel_state(&mut self, panel_id: &str, state: &PanelState) -> Result<()> {
        if let Some(ref mut manager) = self.panel_manager {
            manager.set_panel_state(panel_id, state)
        } else {
            tracing::warn!("No panel manager available for restoring panel state");
            Ok(())
        }
    }

    pub fn restore_pane(&mut self, pane_info: &PaneInfo) -> Result<()> {
        if let Some(ref mut manager) = self.panel_manager {
            manager.restore_pane(pane_info)
        } else {
            tracing::warn!("No panel manager available for restoring pane");
            Ok(())
        }
    }

    pub fn use_default_layout(&mut self) -> Result<()> {
        if let Some(ref mut manager) = self.window_manager {
            manager.use_default_layout()
        } else {
            Ok(())
        }
    }

    pub async fn restore_editor(&mut self, editor_state: &EditorState) -> Result<()> {
        if let Some(ref mut manager) = self.editor_manager {
            manager.restore_editor(editor_state).await
        } else {
            tracing::warn!("No editor manager available for restoring editor");
            Ok(())
        }
    }

    pub async fn prepare_connection(&mut self, profile: &ConnectionProfile) -> Result<()> {
        if let Some(ref mut manager) = self.connection_manager {
            manager.prepare_connection(profile).await
        } else {
            tracing::warn!("No connection manager available for preparing connection");
            Ok(())
        }
    }

    pub fn apply_user_preferences(&mut self, preferences: &UserPreferences) -> Result<()> {
        // Apply preferences to all managers
        if let Some(ref mut manager) = self.window_manager {
            manager.apply_preferences(preferences)?;
        }
        if let Some(ref mut manager) = self.panel_manager {
            manager.apply_preferences(preferences)?;
        }
        if let Some(ref mut manager) = self.editor_manager {
            manager.apply_preferences(preferences)?;
        }
        Ok(())
    }

    pub fn set_query_history(&mut self, history: &[QueryHistoryEntry]) -> Result<()> {
        if let Some(ref mut manager) = self.editor_manager {
            manager.set_query_history(history)
        } else {
            Ok(())
        }
    }
}

/// Trait for managing window state
#[async_trait::async_trait]
pub trait WindowManager: Send + Sync {
    fn set_window_bounds(&mut self, bounds: &WindowBounds) -> Result<()>;
    fn use_default_layout(&mut self) -> Result<()>;
    fn apply_preferences(&mut self, preferences: &UserPreferences) -> Result<()>;
}

/// Trait for managing panel state
#[async_trait::async_trait]
pub trait PanelManager: Send + Sync {
    fn set_panel_state(&mut self, panel_id: &str, state: &PanelState) -> Result<()>;
    fn restore_pane(&mut self, pane_info: &PaneInfo) -> Result<()>;
    fn apply_preferences(&mut self, preferences: &UserPreferences) -> Result<()>;
}

/// Trait for managing editor state
#[async_trait::async_trait]
pub trait EditorManager: Send + Sync {
    async fn restore_editor(&mut self, editor_state: &EditorState) -> Result<()>;
    fn apply_preferences(&mut self, preferences: &UserPreferences) -> Result<()>;
    fn set_query_history(&mut self, history: &[QueryHistoryEntry]) -> Result<()>;
}

/// Trait for managing database connections
#[async_trait::async_trait]
pub trait ConnectionManager: Send + Sync {
    async fn prepare_connection(&mut self, profile: &ConnectionProfile) -> Result<()>;
}

/// State change notification system
#[derive(Debug)]
pub struct StateChanges {
    pub change_type: StateChangeType,
    pub affected_components: Vec<String>,
    pub requires_persistence: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl StateChanges {
    pub fn new(change_type: StateChangeType) -> Self {
        Self {
            change_type,
            affected_components: Vec::new(),
            requires_persistence: false,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_components(mut self, components: Vec<String>) -> Self {
        self.affected_components = components;
        self
    }

    pub fn with_persistence(mut self, requires_persistence: bool) -> Self {
        self.requires_persistence = requires_persistence;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateChangeType {
    WorkspaceLayoutChanged,
    UserPreferencesChanged,
    ConnectionAdded,
    ConnectionRemoved,
    EditorOpened,
    EditorClosed,
    QueryExecuted,
}

/// Trait for observing state changes
#[async_trait::async_trait]
pub trait StateObserver: Send + Sync {
    fn id(&self) -> &str;
    async fn notify_state_change(&self, changes: &StateChanges) -> Result<()>;
}

/// Session snapshot for backup and restoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub workspace_layouts: HashMap<String, WorkspaceLayout>,
    pub active_workspace: Option<String>,
    pub open_editors: Vec<EditorState>,
    pub recent_connections: Vec<ConnectionId>,
    pub user_preferences: UserPreferences,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// State integrity report
#[derive(Debug, Clone)]
pub struct IntegrityReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl IntegrityReport {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
    
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
    
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
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

    /// Get the default state file path
    fn get_state_file_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| CoreError::StateError("Could not determine config directory".to_string()))?;
        
        let tottho_dir = config_dir.join("tottho");
        Ok(tottho_dir.join("state.toml"))
    }

    /// Get the backup state file path
    fn get_backup_state_file_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| CoreError::StateError("Could not determine config directory".to_string()))?;
        
        let tottho_dir = config_dir.join("tottho");
        Ok(tottho_dir.join("state.backup.toml"))
    }

    /// Ensure the state directory exists
    async fn ensure_state_directory() -> Result<()> {
        let state_file_path = Self::get_state_file_path()?;
        if let Some(parent) = state_file_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| CoreError::StateError(format!("Failed to create state directory: {}", e)))?;
        }
        Ok(())
    }

    /// Load application state from storage with corruption recovery
    pub async fn load() -> Result<Self> {
        let state_file_path = Self::get_state_file_path()?;
        
        tracing::debug!("Loading application state from: {:?}", state_file_path);
        
        // If state file doesn't exist, return default state
        if !state_file_path.exists() {
            tracing::info!("State file not found, creating new default state");
            return Ok(Self::new());
        }

        // Try to load the main state file
        match Self::load_and_validate_from_file(&state_file_path).await {
            Ok(state) => {
                tracing::info!("Successfully loaded application state");
                Ok(state)
            }
            Err(e) => {
                tracing::warn!("Failed to load main state file: {}", e);
                
                // Try to load from backup
                let backup_path = Self::get_backup_state_file_path()?;
                if backup_path.exists() {
                    tracing::info!("Attempting to load from backup state file");
                    match Self::load_and_validate_from_file(&backup_path).await {
                        Ok(state) => {
                            tracing::info!("Successfully loaded from backup state");
                            // Save the recovered state as the main state
                            if let Err(save_err) = state.save().await {
                                tracing::warn!("Failed to save recovered state: {}", save_err);
                            }
                            Ok(state)
                        }
                        Err(backup_err) => {
                            tracing::error!("Failed to load backup state: {}", backup_err);
                            
                            // Try to load corrupted state and recover what we can
                            if let Ok(corrupted_state) = Self::load_corrupted_state(&state_file_path).await {
                                tracing::info!("Attempting to recover from corrupted state");
                                match corrupted_state.handle_corruption() {
                                    Ok(recovered_state) => {
                                        tracing::info!("Successfully recovered from corrupted state");
                                        // Save the recovered state
                                        if let Err(save_err) = recovered_state.save().await {
                                            tracing::warn!("Failed to save recovered state: {}", save_err);
                                        }
                                        return Ok(recovered_state);
                                    }
                                    Err(recovery_err) => {
                                        tracing::error!("Failed to recover from corrupted state: {}", recovery_err);
                                    }
                                }
                            }
                            
                            // All recovery attempts failed, return default state
                            tracing::warn!("All recovery attempts failed, using default state");
                            Ok(Self::new())
                        }
                    }
                } else {
                    tracing::warn!("No backup state file found, attempting corruption recovery");
                    
                    // Try to load corrupted state and recover what we can
                    if let Ok(corrupted_state) = Self::load_corrupted_state(&state_file_path).await {
                        tracing::info!("Attempting to recover from corrupted state");
                        match corrupted_state.handle_corruption() {
                            Ok(recovered_state) => {
                                tracing::info!("Successfully recovered from corrupted state");
                                // Save the recovered state
                                if let Err(save_err) = recovered_state.save().await {
                                    tracing::warn!("Failed to save recovered state: {}", save_err);
                                }
                                return Ok(recovered_state);
                            }
                            Err(recovery_err) => {
                                tracing::error!("Failed to recover from corrupted state: {}", recovery_err);
                            }
                        }
                    }
                    
                    tracing::warn!("Using default state");
                    Ok(Self::new())
                }
            }
        }
    }

    /// Load state from a specific file
    async fn load_from_file(path: &Path) -> Result<Self> {
        let mut file = fs::File::open(path).await
            .map_err(|e| CoreError::StateError(format!("Failed to open state file: {}", e)))?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents).await
            .map_err(|e| CoreError::StateError(format!("Failed to read state file: {}", e)))?;
        
        let state: ApplicationState = toml::from_str(&contents)
            .map_err(|e| CoreError::StateError(format!("Failed to parse state file: {}", e)))?;
        
        Ok(state)
    }

    /// Load and validate state from a specific file
    async fn load_and_validate_from_file(path: &Path) -> Result<Self> {
        let state = Self::load_from_file(path).await?;
        
        // Perform integrity check
        let integrity_report = state.check_integrity()?;
        
        if !integrity_report.is_valid() {
            return Err(CoreError::StateCorruption {
                reason: format!("State validation failed: {:?}", integrity_report.errors),
            });
        }
        
        if integrity_report.has_warnings() {
            tracing::warn!("State loaded with warnings: {:?}", integrity_report.warnings);
        }
        
        Ok(state)
    }

    /// Load corrupted state without validation (for recovery purposes)
    async fn load_corrupted_state(path: &Path) -> Result<Self> {
        let mut file = fs::File::open(path).await
            .map_err(|e| CoreError::StateError(format!("Failed to open corrupted state file: {}", e)))?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents).await
            .map_err(|e| CoreError::StateError(format!("Failed to read corrupted state file: {}", e)))?;
        
        // Try to parse as much as possible, using defaults for missing fields
        match toml::from_str::<ApplicationState>(&contents) {
            Ok(state) => Ok(state),
            Err(_) => {
                // If TOML parsing fails completely, try to extract individual sections
                tracing::warn!("TOML parsing failed, attempting partial recovery");
                Self::recover_from_partial_toml(&contents)
            }
        }
    }

    /// Attempt to recover state from partially corrupted TOML
    fn recover_from_partial_toml(contents: &str) -> Result<Self> {
        let mut state = Self::new();
        
        // Try to extract user preferences
        if let Some(prefs_section) = Self::extract_toml_section(contents, "user_preferences") {
            if let Ok(prefs) = toml::from_str::<UserPreferences>(&format!("[user_preferences]\n{}", prefs_section)) {
                if Self::is_valid_user_preferences(&prefs) {
                    state.user_preferences = prefs;
                    tracing::info!("Recovered user preferences from corrupted state");
                }
            }
        }
        
        // Try to extract connection profiles
        if let Some(profiles_section) = Self::extract_toml_section(contents, "connection_profiles") {
            if let Ok(profiles) = toml::from_str::<Vec<ConnectionProfile>>(&format!("connection_profiles = {}", profiles_section)) {
                for profile in profiles {
                    if Self::is_valid_connection_profile(&profile) {
                        state.connection_profiles.push(profile);
                    }
                }
                tracing::info!("Recovered {} connection profiles from corrupted state", state.connection_profiles.len());
            }
        }
        
        Ok(state)
    }

    /// Extract a TOML section from corrupted content
    fn extract_toml_section(contents: &str, section_name: &str) -> Option<String> {
        let section_start = format!("{} = ", section_name);
        let lines: Vec<&str> = contents.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            let trimmed_line = line.trim();
            if trimmed_line.starts_with(&section_start) {
                // Found the section, extract the value part
                let mut section_content = String::new();
                
                // Get the content after the "section_name = " part
                if let Some(content) = trimmed_line.strip_prefix(&section_start) {
                    section_content.push_str(content);
                    
                    // If the content starts with '[' or '{', we need to find the matching closing bracket
                    let content_trimmed = content.trim();
                    if content_trimmed.starts_with('[') || content_trimmed.starts_with('{') {
                        let opening_char = content_trimmed.chars().next().unwrap();
                        let closing_char = if opening_char == '[' { ']' } else { '}' };
                        let mut bracket_count = 0;
                        let mut found_complete = false;
                        
                        // Count brackets in the first line
                        for ch in content.chars() {
                            if ch == opening_char {
                                bracket_count += 1;
                            } else if ch == closing_char {
                                bracket_count -= 1;
                                if bracket_count == 0 {
                                    found_complete = true;
                                    break;
                                }
                            }
                        }
                        
                        // If not complete, continue reading lines
                        if !found_complete {
                            for j in (i + 1)..lines.len() {
                                let current_line = lines[j];
                                section_content.push('\n');
                                section_content.push_str(current_line);
                                
                                // Count brackets in this line
                                for ch in current_line.chars() {
                                    if ch == opening_char {
                                        bracket_count += 1;
                                    } else if ch == closing_char {
                                        bracket_count -= 1;
                                        if bracket_count == 0 {
                                            found_complete = true;
                                            break;
                                        }
                                    }
                                }
                                
                                if found_complete {
                                    break;
                                }
                                
                                // Stop if we hit another top-level section
                                if current_line.trim().contains(" = ") && !current_line.trim().starts_with(' ') && !current_line.trim().starts_with('\t') {
                                    break;
                                }
                            }
                        }
                    }
                }
                
                return Some(section_content);
            }
        }
        
        None
    }

    /// Save application state to storage with automatic backup
    pub async fn save(&self) -> Result<()> {
        // Validate state before saving
        let integrity_report = self.check_integrity()?;
        if !integrity_report.is_valid() {
            return Err(CoreError::StateCorruption {
                reason: format!("Cannot save invalid state: {:?}", integrity_report.errors),
            });
        }
        
        if integrity_report.has_warnings() {
            tracing::warn!("Saving state with warnings: {:?}", integrity_report.warnings);
        }
        
        Self::ensure_state_directory().await?;
        
        let state_file_path = Self::get_state_file_path()?;
        
        tracing::debug!("Saving application state to: {:?}", state_file_path);
        
        // Create backup before saving new state
        if state_file_path.exists() {
            if let Err(e) = self.create_backup().await {
                tracing::warn!("Failed to create backup before saving: {}", e);
                // Continue with save operation even if backup fails
            }
        }
        
        // Serialize state to TOML
        let toml_content = toml::to_string_pretty(self)
            .map_err(|e| CoreError::StateError(format!("Failed to serialize state: {}", e)))?;
        
        // Write to temporary file first, then rename for atomic operation
        let temp_file_path = state_file_path.with_extension("tmp");
        
        let mut file = fs::File::create(&temp_file_path).await
            .map_err(|e| CoreError::StateError(format!("Failed to create temp state file: {}", e)))?;
        
        file.write_all(toml_content.as_bytes()).await
            .map_err(|e| CoreError::StateError(format!("Failed to write state file: {}", e)))?;
        
        file.sync_all().await
            .map_err(|e| CoreError::StateError(format!("Failed to sync state file: {}", e)))?;
        
        // Atomically replace the old file with the new one
        fs::rename(&temp_file_path, &state_file_path).await
            .map_err(|e| CoreError::StateError(format!("Failed to replace state file: {}", e)))?;
        
        tracing::info!("Successfully saved application state");
        Ok(())
    }

    /// Validate the state for consistency and integrity (legacy method)
    fn validate(&self) -> Result<()> {
        let integrity_report = self.check_integrity()?;
        
        if !integrity_report.is_valid() {
            return Err(CoreError::StateCorruption {
                reason: format!("State validation failed: {:?}", integrity_report.errors),
            });
        }
        
        if integrity_report.has_warnings() {
            tracing::warn!("State validation completed with warnings: {:?}", integrity_report.warnings);
        }
        
        tracing::debug!("State validation passed");
        Ok(())
    }

    /// Restore session state including workspace layouts and editor states
    pub async fn restore_session(&self, context: &mut SessionRestorationContext) -> Result<()> {
        tracing::info!("Starting session restoration");
        
        // Restore workspace layouts
        self.restore_workspace_layouts(context).await?;
        
        // Restore editor states
        self.restore_editor_states(context).await?;
        
        // Restore recent connections
        self.restore_recent_connections(context).await?;
        
        // Restore UI state
        self.restore_ui_state(context).await?;
        
        tracing::info!("Session restoration completed successfully");
        Ok(())
    }

    /// Restore workspace layouts
    async fn restore_workspace_layouts(&self, context: &mut SessionRestorationContext) -> Result<()> {
        tracing::debug!("Restoring workspace layouts");
        
        // Get the last active workspace or default
        let default_workspace = "default".to_string();
        let workspace_id = self.session_data.last_workspace
            .as_ref()
            .unwrap_or(&default_workspace);
        
        if let Some(layout) = self.workspace_layouts.get(workspace_id) {
            // Restore window bounds
            context.set_window_bounds(&layout.window_bounds)?;
            
            // Restore panel states
            for (panel_id, panel_state) in &layout.panel_states {
                context.set_panel_state(panel_id, panel_state)?;
            }
            
            // Restore active panes
            for pane_info in &layout.active_panes {
                context.restore_pane(pane_info)?;
            }
            
            tracing::info!("Restored workspace layout: {}", workspace_id);
        } else {
            tracing::warn!("Workspace layout not found: {}, using default", workspace_id);
            context.use_default_layout()?;
        }
        
        Ok(())
    }

    /// Restore editor states
    async fn restore_editor_states(&self, context: &mut SessionRestorationContext) -> Result<()> {
        tracing::debug!("Restoring editor states");
        
        for editor_state in &self.session_data.open_editors {
            match context.restore_editor(editor_state).await {
                Ok(_) => {
                    tracing::debug!("Restored editor: {}", editor_state.id);
                }
                Err(e) => {
                    tracing::warn!("Failed to restore editor {}: {}", editor_state.id, e);
                    // Continue with other editors even if one fails
                }
            }
        }
        
        tracing::info!("Restored {} editors", self.session_data.open_editors.len());
        Ok(())
    }

    /// Restore recent connections
    async fn restore_recent_connections(&self, context: &mut SessionRestorationContext) -> Result<()> {
        tracing::debug!("Restoring recent connections");
        
        for connection_id in &self.session_data.recent_connections {
            if let Some(profile) = self.get_connection_profile(connection_id.clone()) {
                match context.prepare_connection(profile).await {
                    Ok(_) => {
                        tracing::debug!("Prepared connection: {}", connection_id);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to prepare connection {}: {}", connection_id, e);
                        // Continue with other connections
                    }
                }
            } else {
                tracing::warn!("Connection profile not found: {}", connection_id);
            }
        }
        
        Ok(())
    }

    /// Restore UI state
    async fn restore_ui_state(&self, context: &mut SessionRestorationContext) -> Result<()> {
        tracing::debug!("Restoring UI state");
        
        // Apply user preferences
        context.apply_user_preferences(&self.user_preferences)?;
        
        // Restore query history (make it available to UI components)
        context.set_query_history(&self.session_data.query_history)?;
        
        tracing::info!("UI state restored");
        Ok(())
    }

    /// Synchronize state changes across application components
    pub async fn synchronize_state(&self, changes: StateChanges, observers: &[Box<dyn StateObserver>]) -> Result<()> {
        tracing::debug!("Synchronizing state changes: {:?}", changes);
        
        // Notify all registered state observers
        for observer in observers {
            match observer.notify_state_change(&changes).await {
                Ok(_) => {
                    tracing::debug!("Notified observer: {}", observer.id());
                }
                Err(e) => {
                    tracing::warn!("Failed to notify observer {}: {}", observer.id(), e);
                }
            }
        }
        
        // Update internal state if needed
        if changes.requires_persistence {
            self.save().await?;
        }
        
        Ok(())
    }

    /// Create a session snapshot for restoration
    pub fn create_session_snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            workspace_layouts: self.workspace_layouts.clone(),
            active_workspace: self.session_data.last_workspace.clone(),
            open_editors: self.session_data.open_editors.clone(),
            recent_connections: self.session_data.recent_connections.clone(),
            user_preferences: self.user_preferences.clone(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Restore from a session snapshot
    pub async fn restore_from_snapshot(&mut self, snapshot: SessionSnapshot) -> Result<()> {
        tracing::info!("Restoring from session snapshot");
        
        // Validate snapshot age (don't restore very old snapshots)
        let age = chrono::Utc::now() - snapshot.timestamp;
        if age > chrono::Duration::hours(24) {
            tracing::warn!("Session snapshot is old ({} hours), using with caution", age.num_hours());
        }
        
        // Restore workspace layouts
        self.workspace_layouts = snapshot.workspace_layouts;
        
        // Restore session data
        self.session_data.last_workspace = snapshot.active_workspace;
        self.session_data.open_editors = snapshot.open_editors;
        self.session_data.recent_connections = snapshot.recent_connections;
        
        // Restore user preferences (with validation)
        if Self::is_valid_user_preferences(&snapshot.user_preferences) {
            self.user_preferences = snapshot.user_preferences;
        } else {
            tracing::warn!("Invalid user preferences in snapshot, keeping current preferences");
        }
        
        tracing::info!("Session snapshot restored successfully");
        Ok(())
    }

    /// Handle state corruption by falling back to defaults
    pub fn handle_corruption(&self) -> Result<Self> {
        tracing::warn!("State corruption detected, falling back to defaults");
        
        // Try to preserve some user data if possible
        let mut new_state = Self::new();
        
        // Attempt to preserve valid connection profiles
        for profile in &self.connection_profiles {
            if Self::is_valid_connection_profile(profile) {
                new_state.connection_profiles.push(profile.clone());
                tracing::info!("Preserved connection profile: {}", profile.name);
            }
        }
        
        // Attempt to preserve valid user preferences
        if Self::is_valid_user_preferences(&self.user_preferences) {
            new_state.user_preferences = self.user_preferences.clone();
            tracing::info!("Preserved user preferences");
        }
        
        // Attempt to preserve recent query history (last 10 queries)
        let valid_queries: Vec<_> = self.session_data.query_history
            .iter()
            .rev()
            .take(10)
            .filter(|entry| Self::is_valid_query_history_entry(entry))
            .cloned()
            .collect();
        
        if !valid_queries.is_empty() {
            new_state.session_data.query_history = valid_queries.into_iter().rev().collect();
            tracing::info!("Preserved {} recent queries", new_state.session_data.query_history.len());
        }
        
        Ok(new_state)
    }

    /// Create a backup of the current state
    pub async fn create_backup(&self) -> Result<()> {
        let backup_path = Self::get_backup_state_file_path()?;
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let timestamped_backup = backup_path.with_file_name(format!("state.backup.{}.toml", timestamp));
        
        tracing::debug!("Creating state backup at: {:?}", timestamped_backup);
        
        // Ensure backup directory exists
        if let Some(parent) = timestamped_backup.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| CoreError::StateError(format!("Failed to create backup directory: {}", e)))?;
        }
        
        // Serialize and save backup
        let toml_content = toml::to_string_pretty(self)
            .map_err(|e| CoreError::StateError(format!("Failed to serialize state for backup: {}", e)))?;
        
        fs::write(&timestamped_backup, toml_content).await
            .map_err(|e| CoreError::StateError(format!("Failed to write backup file: {}", e)))?;
        
        // Also update the main backup file
        if let Err(e) = fs::copy(&timestamped_backup, &backup_path).await {
            tracing::warn!("Failed to update main backup file: {}", e);
        }
        
        // Clean up old backups (keep last 5)
        self.cleanup_old_backups().await;
        
        tracing::info!("State backup created successfully");
        Ok(())
    }

    /// Restore state from backup
    pub async fn restore_from_backup() -> Result<Self> {
        let backup_path = Self::get_backup_state_file_path()?;
        
        if !backup_path.exists() {
            return Err(CoreError::StateError("No backup file found".to_string()));
        }
        
        tracing::info!("Restoring state from backup: {:?}", backup_path);
        
        let state = Self::load_from_file(&backup_path).await?;
        
        // Validate the restored state
        state.validate()?;
        
        tracing::info!("State restored from backup successfully");
        Ok(state)
    }

    /// Clean up old backup files, keeping only the most recent ones
    async fn cleanup_old_backups(&self) {
        let backup_dir = match Self::get_backup_state_file_path() {
            Ok(path) => match path.parent() {
                Some(dir) => dir.to_path_buf(),
                None => return,
            },
            Err(_) => return,
        };
        
        let mut backup_files = Vec::new();
        
        if let Ok(mut entries) = fs::read_dir(&backup_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("state.backup.") && name.ends_with(".toml") {
                        if let Ok(metadata) = entry.metadata().await {
                            backup_files.push((path, metadata.modified().unwrap_or(std::time::UNIX_EPOCH)));
                        }
                    }
                }
            }
        }
        
        // Sort by modification time (newest first)
        backup_files.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Remove old backups (keep only 5 most recent)
        for (path, _) in backup_files.into_iter().skip(5) {
            if let Err(e) = fs::remove_file(&path).await {
                tracing::warn!("Failed to remove old backup file {:?}: {}", path, e);
            } else {
                tracing::debug!("Removed old backup file: {:?}", path);
            }
        }
    }

    /// Perform comprehensive state integrity check
    pub fn check_integrity(&self) -> Result<IntegrityReport> {
        let mut report = IntegrityReport::new();
        
        // Check version
        if self.version.is_empty() {
            report.add_error("Missing version information".to_string());
        }
        
        // Check workspace layouts
        for (id, layout) in &self.workspace_layouts {
            if layout.id != *id {
                report.add_error(format!("Workspace layout ID mismatch: {} != {}", layout.id, id));
            }
            
            if layout.window_bounds.width == 0 || layout.window_bounds.height == 0 {
                report.add_warning(format!("Invalid window bounds for workspace {}", id));
            }
            
            if layout.window_bounds.width > 10000 || layout.window_bounds.height > 10000 {
                report.add_warning(format!("Suspicious window bounds for workspace {}", id));
            }
        }
        
        // Check connection profiles
        for profile in &self.connection_profiles {
            if !Self::is_valid_connection_profile(profile) {
                report.add_error(format!("Invalid connection profile: {}", profile.name));
            }
        }
        
        // Check user preferences
        if !Self::is_valid_user_preferences(&self.user_preferences) {
            report.add_error("Invalid user preferences".to_string());
        }
        
        // Check session data
        for entry in &self.session_data.query_history {
            if !Self::is_valid_query_history_entry(entry) {
                report.add_warning(format!("Invalid query history entry: {}", entry.id));
            }
        }
        
        // Check for duplicate connection IDs
        let mut connection_ids = std::collections::HashSet::new();
        for profile in &self.connection_profiles {
            if !connection_ids.insert(&profile.id) {
                report.add_error(format!("Duplicate connection ID: {}", profile.id));
            }
        }
        
        // Check for orphaned references
        let valid_connection_ids: std::collections::HashSet<_> = 
            self.connection_profiles.iter().map(|p| &p.id).collect();
        
        for connection_id in &self.session_data.recent_connections {
            if !valid_connection_ids.contains(connection_id) {
                report.add_warning(format!("Orphaned connection reference: {}", connection_id));
            }
        }
        
        for entry in &self.session_data.query_history {
            if !valid_connection_ids.contains(&entry.connection_id) {
                report.add_warning(format!("Query history references unknown connection: {}", entry.connection_id));
            }
        }
        
        tracing::debug!("State integrity check completed: {} errors, {} warnings", 
                       report.errors.len(), report.warnings.len());
        
        Ok(report)
    }

    /// Validate a connection profile
    fn is_valid_connection_profile(profile: &ConnectionProfile) -> bool {
        !profile.id.is_empty() 
            && !profile.name.is_empty()
            && !profile.host.is_empty()
            && profile.port > 0
            && profile.port <= 65535
            && !profile.database_type.is_empty()
    }

    /// Validate user preferences
    fn is_valid_user_preferences(prefs: &UserPreferences) -> bool {
        prefs.font_size > 0 
            && prefs.font_size <= 72
            && prefs.tab_size > 0
            && prefs.tab_size <= 16
            && prefs.query_timeout > 0
            && prefs.query_timeout <= 3600
            && !prefs.theme.is_empty()
            && !prefs.font_family.is_empty()
    }

    /// Validate a query history entry
    fn is_valid_query_history_entry(entry: &QueryHistoryEntry) -> bool {
        !entry.id.is_empty()
            && !entry.query.trim().is_empty()
            && !entry.connection_id.is_empty()
            && entry.query.len() <= 1_000_000 // Reasonable query size limit
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

    /// Set the active workspace
    pub fn set_active_workspace(&mut self, workspace_id: String) {
        self.session_data.last_workspace = Some(workspace_id);
    }

    /// Add an editor to the session
    pub fn add_editor_state(&mut self, editor_state: EditorState) {
        // Remove existing editor with same ID if present
        self.session_data.open_editors.retain(|e| e.id != editor_state.id);
        self.session_data.open_editors.push(editor_state);
    }

    /// Remove an editor from the session
    pub fn remove_editor_state(&mut self, editor_id: &str) -> bool {
        let initial_len = self.session_data.open_editors.len();
        self.session_data.open_editors.retain(|e| e.id != editor_id);
        self.session_data.open_editors.len() < initial_len
    }

    /// Update recent connections list
    pub fn update_recent_connections(&mut self, connection_id: ConnectionId) {
        // Remove if already present
        self.session_data.recent_connections.retain(|id| id != &connection_id);
        
        // Add to front
        self.session_data.recent_connections.insert(0, connection_id);
        
        // Keep only last 10 connections
        if self.session_data.recent_connections.len() > 10 {
            self.session_data.recent_connections.truncate(10);
        }
    }

    /// Get the most recently used connection
    pub fn get_most_recent_connection(&self) -> Option<&ConnectionId> {
        self.session_data.recent_connections.first()
    }

    /// Check if session has restorable content
    pub fn has_restorable_session(&self) -> bool {
        !self.workspace_layouts.is_empty() 
            || !self.session_data.open_editors.is_empty()
            || !self.session_data.recent_connections.is_empty()
    }
}

impl Default for ApplicationState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;
    use tokio::fs;

    fn create_test_state() -> ApplicationState {
        let mut state = ApplicationState::new();
        
        // Add test connection profile
        state.add_connection_profile(ConnectionProfile {
            id: "test_conn_1".to_string(),
            name: "Test Connection".to_string(),
            database_type: "postgresql".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testuser".to_string(),
            ssl_mode: "prefer".to_string(),
            created_at: chrono::Utc::now(),
            last_used: Some(chrono::Utc::now()),
        });
        
        // Add test workspace layout
        let workspace_layout = WorkspaceLayout {
            id: "default".to_string(),
            window_bounds: WindowBounds {
                x: 100,
                y: 100,
                width: 1200,
                height: 800,
                maximized: false,
            },
            panel_states: HashMap::new(),
            active_panes: Vec::new(),
        };
        state.update_workspace_layout(workspace_layout);
        
        // Add test query history
        state.add_query_history(QueryHistoryEntry {
            id: "query_1".to_string(),
            query: "SELECT * FROM users".to_string(),
            connection_id: "test_conn_1".to_string(),
            executed_at: chrono::Utc::now(),
            execution_time: Some(150),
            success: true,
            error_message: None,
        });
        
        state
    }

    #[tokio::test]
    async fn test_state_integrity_check() {
        let state = create_test_state();
        let report = state.check_integrity().unwrap();
        
        assert!(report.is_valid(), "State should be valid");
        assert!(!report.has_warnings(), "State should have no warnings");
    }

    #[tokio::test]
    async fn test_state_integrity_check_with_errors() {
        let mut state = create_test_state();
        
        // Add invalid connection profile
        state.connection_profiles.push(ConnectionProfile {
            id: "".to_string(), // Invalid empty ID
            name: "Invalid Connection".to_string(),
            database_type: "postgresql".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testuser".to_string(),
            ssl_mode: "prefer".to_string(),
            created_at: chrono::Utc::now(),
            last_used: None,
        });
        
        let report = state.check_integrity().unwrap();
        
        assert!(!report.is_valid(), "State should be invalid");
        assert!(!report.errors.is_empty(), "Should have errors");
    }

    #[tokio::test]
    async fn test_corruption_recovery() {
        let mut corrupted_state = create_test_state();
        
        // Add some invalid data to simulate corruption
        corrupted_state.connection_profiles.push(ConnectionProfile {
            id: "".to_string(), // Invalid
            name: "".to_string(), // Invalid
            database_type: "postgresql".to_string(),
            host: "".to_string(), // Invalid
            port: 0, // Invalid
            database: "testdb".to_string(),
            username: "testuser".to_string(),
            ssl_mode: "prefer".to_string(),
            created_at: chrono::Utc::now(),
            last_used: None,
        });
        
        // Corrupt user preferences
        corrupted_state.user_preferences.font_size = 0; // Invalid
        
        let recovered_state = corrupted_state.handle_corruption().unwrap();
        
        // Check that valid data was preserved
        assert_eq!(recovered_state.connection_profiles.len(), 1, "Should preserve valid connection");
        assert_eq!(recovered_state.connection_profiles[0].id, "test_conn_1");
        
        // Check that invalid preferences were reset to defaults
        assert_eq!(recovered_state.user_preferences.font_size, 14, "Should use default font size");
        
        // Check that query history was preserved
        assert!(!recovered_state.session_data.query_history.is_empty(), "Should preserve query history");
    }

    #[tokio::test]
    async fn test_validation_functions() {
        // Test valid connection profile
        let valid_profile = ConnectionProfile {
            id: "test_id".to_string(),
            name: "Test".to_string(),
            database_type: "postgresql".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testuser".to_string(),
            ssl_mode: "prefer".to_string(),
            created_at: chrono::Utc::now(),
            last_used: None,
        };
        assert!(ApplicationState::is_valid_connection_profile(&valid_profile));
        
        // Test invalid connection profile
        let invalid_profile = ConnectionProfile {
            id: "".to_string(), // Invalid
            name: "Test".to_string(),
            database_type: "postgresql".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testuser".to_string(),
            ssl_mode: "prefer".to_string(),
            created_at: chrono::Utc::now(),
            last_used: None,
        };
        assert!(!ApplicationState::is_valid_connection_profile(&invalid_profile));
        
        // Test valid user preferences
        let valid_prefs = UserPreferences::default();
        assert!(ApplicationState::is_valid_user_preferences(&valid_prefs));
        
        // Test invalid user preferences
        let mut invalid_prefs = UserPreferences::default();
        invalid_prefs.font_size = 0; // Invalid
        assert!(!ApplicationState::is_valid_user_preferences(&invalid_prefs));
        
        // Test valid query history entry
        let valid_entry = QueryHistoryEntry {
            id: "test_query".to_string(),
            query: "SELECT 1".to_string(),
            connection_id: "test_conn".to_string(),
            executed_at: chrono::Utc::now(),
            execution_time: Some(100),
            success: true,
            error_message: None,
        };
        assert!(ApplicationState::is_valid_query_history_entry(&valid_entry));
        
        // Test invalid query history entry
        let invalid_entry = QueryHistoryEntry {
            id: "".to_string(), // Invalid
            query: "SELECT 1".to_string(),
            connection_id: "test_conn".to_string(),
            executed_at: chrono::Utc::now(),
            execution_time: Some(100),
            success: true,
            error_message: None,
        };
        assert!(!ApplicationState::is_valid_query_history_entry(&invalid_entry));
    }

    #[tokio::test]
    async fn test_backup_and_restore() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("state.toml");
        let _backup_path = temp_dir.path().join("state.backup.toml");
        
        let original_state = create_test_state();
        
        // Save state to temporary file
        let toml_content = toml::to_string_pretty(&original_state).unwrap();
        fs::write(&state_path, toml_content).await.unwrap();
        
        // Load state and create backup
        let loaded_state = ApplicationState::load_from_file(&state_path).await.unwrap();
        loaded_state.create_backup().await.unwrap();
        
        // Verify backup was created (this is a simplified test since we can't easily mock the path functions)
        // In a real scenario, we would need to mock the path functions or use dependency injection
    }

    #[tokio::test]
    async fn test_session_restoration_context() {
        let mut context = SessionRestorationContext::new();
        
        // Test that methods work without managers (should not panic)
        let bounds = WindowBounds {
            x: 100,
            y: 100,
            width: 800,
            height: 600,
            maximized: false,
        };
        assert!(context.set_window_bounds(&bounds).is_ok());
        
        let panel_state = PanelState {
            visible: true,
            width: Some(300),
            height: None,
            position: PanelPosition::Left,
        };
        assert!(context.set_panel_state("test_panel", &panel_state).is_ok());
        
        let prefs = UserPreferences::default();
        assert!(context.apply_user_preferences(&prefs).is_ok());
    }

    #[tokio::test]
    async fn test_session_snapshot() {
        let state = create_test_state();
        let snapshot = state.create_session_snapshot();
        
        assert_eq!(snapshot.workspace_layouts.len(), 1);
        assert!(snapshot.workspace_layouts.contains_key("default"));
        assert_eq!(snapshot.open_editors.len(), 0); // No editors in test state
        assert_eq!(snapshot.recent_connections.len(), 0); // No recent connections in test state
        
        // Test restoring from snapshot
        let mut new_state = ApplicationState::new();
        new_state.restore_from_snapshot(snapshot).await.unwrap();
        
        assert_eq!(new_state.workspace_layouts.len(), 1);
        assert!(new_state.workspace_layouts.contains_key("default"));
    }

    #[tokio::test]
    async fn test_session_management_methods() {
        let mut state = ApplicationState::new();
        
        // Test workspace management
        state.set_active_workspace("test_workspace".to_string());
        assert_eq!(state.session_data.last_workspace, Some("test_workspace".to_string()));
        
        // Test editor management
        let editor_state = EditorState {
            id: "editor_1".to_string(),
            editor_type: "sql".to_string(),
            connection_id: Some("conn_1".to_string()),
            content: "SELECT * FROM users".to_string(),
            cursor_position: CursorPosition { line: 1, column: 1 },
        };
        
        state.add_editor_state(editor_state.clone());
        assert_eq!(state.session_data.open_editors.len(), 1);
        assert_eq!(state.session_data.open_editors[0].id, "editor_1");
        
        // Test removing editor
        assert!(state.remove_editor_state("editor_1"));
        assert_eq!(state.session_data.open_editors.len(), 0);
        assert!(!state.remove_editor_state("nonexistent"));
        
        // Test recent connections
        state.update_recent_connections("conn_1".to_string());
        state.update_recent_connections("conn_2".to_string());
        state.update_recent_connections("conn_1".to_string()); // Should move to front
        
        assert_eq!(state.session_data.recent_connections.len(), 2);
        assert_eq!(state.get_most_recent_connection(), Some(&"conn_1".to_string()));
        
        // Test restorable session check
        assert!(state.has_restorable_session()); // Has recent connections
        
        let empty_state = ApplicationState::new();
        assert!(!empty_state.has_restorable_session());
    }

    #[tokio::test]
    async fn test_state_changes() {
        let changes = StateChanges::new(StateChangeType::WorkspaceLayoutChanged)
            .with_components(vec!["workspace".to_string(), "panels".to_string()])
            .with_persistence(true);
        
        assert_eq!(changes.change_type, StateChangeType::WorkspaceLayoutChanged);
        assert_eq!(changes.affected_components.len(), 2);
        assert!(changes.requires_persistence);
    }

    #[test]
    fn test_toml_section_extraction() {
        let corrupted_toml = r#"
version = "1.0.0"
user_preferences = { theme = "dark", font_size = 14 }
connection_profiles = [
    { id = "conn1", name = "Test", database_type = "postgresql", host = "localhost", port = 5432, database = "test", username = "user", ssl_mode = "prefer", created_at = "2023-01-01T00:00:00Z" }
]
workspace_layouts = {}
"#;
        
        let prefs_section = ApplicationState::extract_toml_section(corrupted_toml, "user_preferences");
        assert!(prefs_section.is_some());
        let prefs_content = prefs_section.unwrap();
        assert!(prefs_content.contains("theme = \"dark\""));
        assert!(prefs_content.contains("font_size = 14"));
        
        let profiles_section = ApplicationState::extract_toml_section(corrupted_toml, "connection_profiles");
        assert!(profiles_section.is_some());
        let profiles_content = profiles_section.unwrap();
        assert!(profiles_content.contains("conn1"));
        assert!(profiles_content.contains("id = \"conn1\""));
        
        let layouts_section = ApplicationState::extract_toml_section(corrupted_toml, "workspace_layouts");
        assert!(layouts_section.is_some());
        assert_eq!(layouts_section.unwrap().trim(), "{}");
    }
}