//! Workspace management for Tottho application
//! 
//! Handles workspace layouts, session management, and window state persistence
//! following Zed's workspace patterns.

use anyhow::Result;
use gpui::{App, AppContext, Context, Entity, Global, WeakEntity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::{CoreError, TotthoConfig};

/// Workspace manager for handling layouts and session management
#[derive(Clone)]
pub struct WorkspaceManager {
    workspaces: Arc<RwLock<HashMap<WorkspaceId, Workspace>>>,
    active_workspace: Arc<RwLock<Option<WorkspaceId>>>,
    config: Arc<RwLock<crate::core::config::WorkspaceConfig>>,
}

/// Unique identifier for workspaces
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub String);

impl WorkspaceId {
    pub fn new() -> Self {
        Self(cuid2::create_id())
    }

    pub fn from_path(path: &PathBuf) -> Self {
        Self(format!("path:{}", path.display()))
    }
}

// Use WorkspaceConfig and WindowSize from config.rs to avoid duplication
use crate::core::config::{WorkspaceConfig, WindowSize};

/// Individual workspace containing layout and state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub path: Option<PathBuf>,
    pub layout: WorkspaceLayout,
    pub window_state: WindowState,
    pub database_connections: Vec<String>,
    pub open_tabs: Vec<TabInfo>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
}

/// Workspace layout information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    pub layout_type: String,
    pub panels: Vec<PanelInfo>,
    pub splitters: Vec<SplitterInfo>,
}

impl Default for WorkspaceLayout {
    fn default() -> Self {
        Self {
            layout_type: "default".to_string(),
            panels: vec![
                PanelInfo {
                    id: "database-inspector".to_string(),
                    position: PanelPosition::Left,
                    size: 300,
                    visible: true,
                },
                PanelInfo {
                    id: "query-editor".to_string(),
                    position: PanelPosition::Center,
                    size: 800,
                    visible: true,
                },
                PanelInfo {
                    id: "query-results".to_string(),
                    position: PanelPosition::Bottom,
                    size: 200,
                    visible: true,
                },
            ],
            splitters: Vec::new(),
        }
    }
}

/// Panel information for workspace layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelInfo {
    pub id: String,
    pub position: PanelPosition,
    pub size: u32,
    pub visible: bool,
}

/// Panel position in the workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PanelPosition {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

/// Splitter information for resizable panels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitterInfo {
    pub id: String,
    pub position: f32,
    pub orientation: SplitterOrientation,
}

/// Splitter orientation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SplitterOrientation {
    Horizontal,
    Vertical,
}

/// Window state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
    pub fullscreen: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 1200,
            height: 800,
            maximized: false,
            fullscreen: false,
        }
    }
}

/// Tab information for open tabs in workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: String,
    pub title: String,
    pub tab_type: TabType,
    pub active: bool,
    pub path: Option<PathBuf>,
}

/// Type of tab content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TabType {
    QueryEditor,
    DatabaseInspector,
    Settings,
    Welcome,
}

impl WorkspaceManager {
    /// Create a new workspace manager
    pub fn new() -> Self {
        Self {
            workspaces: Arc::new(RwLock::new(HashMap::new())),
            active_workspace: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(crate::core::config::WorkspaceConfig::default())),
        }
    }

    /// Initialize the workspace manager with configuration
    pub async fn initialize(&self, config: &TotthoConfig) -> Result<(), CoreError> {
        tracing::info!("Initializing workspace manager");

        // Update configuration
        {
            let mut workspace_config = self.config.write().await;
            *workspace_config = config.workspace.clone();
        }

        // Load existing workspaces
        self.load_workspaces().await?;

        tracing::info!("Workspace manager initialized successfully");
        Ok(())
    }

    /// Create a new workspace
    pub async fn create_workspace(
        &self,
        name: String,
        path: Option<PathBuf>,
    ) -> Result<WorkspaceId, CoreError> {
        let workspace_id = if let Some(path) = &path {
            WorkspaceId::from_path(path)
        } else {
            WorkspaceId::new()
        };

        let workspace = Workspace {
            id: workspace_id.clone(),
            name,
            path,
            layout: WorkspaceLayout::default(),
            window_state: WindowState::default(),
            database_connections: Vec::new(),
            open_tabs: Vec::new(),
            last_accessed: chrono::Utc::now(),
        };

        {
            let mut workspaces = self.workspaces.write().await;
            workspaces.insert(workspace_id.clone(), workspace);
        }

        tracing::info!("Created new workspace: {:?}", workspace_id);
        Ok(workspace_id)
    }

    /// Get a workspace by ID
    pub async fn get_workspace(&self, id: &WorkspaceId) -> Option<Workspace> {
        let workspaces = self.workspaces.read().await;
        workspaces.get(id).cloned()
    }

    /// Set the active workspace
    pub async fn set_active_workspace(&self, id: WorkspaceId) -> Result<(), CoreError> {
        // Verify workspace exists
        {
            let workspaces = self.workspaces.read().await;
            if !workspaces.contains_key(&id) {
                return Err(CoreError::StateError(format!(
                    "Workspace not found: {:?}",
                    id
                )));
            }
        }

        // Update last accessed time
        {
            let mut workspaces = self.workspaces.write().await;
            if let Some(workspace) = workspaces.get_mut(&id) {
                workspace.last_accessed = chrono::Utc::now();
            }
        }

        // Set as active
        {
            let mut active = self.active_workspace.write().await;
            *active = Some(id.clone());
        }

        tracing::info!("Set active workspace: {:?}", id);
        Ok(())
    }

    /// Get the active workspace
    pub async fn get_active_workspace(&self) -> Option<Workspace> {
        let active_id = {
            let active = self.active_workspace.read().await;
            active.clone()
        };

        if let Some(id) = active_id {
            self.get_workspace(&id).await
        } else {
            None
        }
    }

    /// Update workspace layout
    pub async fn update_workspace_layout(
        &self,
        id: &WorkspaceId,
        layout: WorkspaceLayout,
    ) -> Result<(), CoreError> {
        let mut workspaces = self.workspaces.write().await;
        if let Some(workspace) = workspaces.get_mut(id) {
            workspace.layout = layout;
            workspace.last_accessed = chrono::Utc::now();
            tracing::debug!("Updated layout for workspace: {:?}", id);
            Ok(())
        } else {
            Err(CoreError::StateError(format!(
                "Workspace not found: {:?}",
                id
            )))
        }
    }

    /// Update window state
    pub async fn update_window_state(
        &self,
        id: &WorkspaceId,
        window_state: WindowState,
    ) -> Result<(), CoreError> {
        let mut workspaces = self.workspaces.write().await;
        if let Some(workspace) = workspaces.get_mut(id) {
            workspace.window_state = window_state;
            workspace.last_accessed = chrono::Utc::now();
            tracing::debug!("Updated window state for workspace: {:?}", id);
            Ok(())
        } else {
            Err(CoreError::StateError(format!(
                "Workspace not found: {:?}",
                id
            )))
        }
    }

    /// List all workspaces
    pub async fn list_workspaces(&self) -> Vec<Workspace> {
        let workspaces = self.workspaces.read().await;
        let mut workspace_list: Vec<Workspace> = workspaces.values().cloned().collect();
        
        // Sort by last accessed time (most recent first)
        workspace_list.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        
        workspace_list
    }

    /// Get recent workspaces
    pub async fn get_recent_workspaces(&self, limit: usize) -> Vec<Workspace> {
        let mut workspaces = self.list_workspaces().await;
        workspaces.truncate(limit);
        workspaces
    }

    /// Remove a workspace
    pub async fn remove_workspace(&self, id: &WorkspaceId) -> Result<(), CoreError> {
        // Check if it's the active workspace
        {
            let active = self.active_workspace.read().await;
            if active.as_ref() == Some(id) {
                drop(active);
                let mut active = self.active_workspace.write().await;
                *active = None;
            }
        }

        // Remove from workspaces
        {
            let mut workspaces = self.workspaces.write().await;
            workspaces.remove(id);
        }

        tracing::info!("Removed workspace: {:?}", id);
        Ok(())
    }

    /// Save workspace state to persistent storage
    pub async fn save_workspace_state(&self) -> Result<(), CoreError> {
        // TODO: Implement persistent storage
        tracing::debug!("Workspace state save placeholder");
        Ok(())
    }

    /// Load workspaces from persistent storage
    pub async fn load_workspaces(&self) -> Result<(), CoreError> {
        // TODO: Implement persistent storage loading
        tracing::debug!("Workspace loading placeholder");
        
        // For now, create a default workspace if none exist
        let workspaces_count = {
            let workspaces = self.workspaces.read().await;
            workspaces.len()
        };

        if workspaces_count == 0 {
            let default_id = self.create_workspace("Default".to_string(), None).await?;
            self.set_active_workspace(default_id).await?;
            tracing::info!("Created default workspace");
        }

        Ok(())
    }

    /// Restore session from previous application run
    pub async fn restore_session(&self) -> Result<(), CoreError> {
        tracing::info!("Restoring workspace session");
        
        // Load workspaces if not already loaded
        self.load_workspaces().await?;
        
        // If no active workspace, set the most recent one as active
        {
            let active = self.active_workspace.read().await;
            if active.is_none() {
                drop(active);
                let recent_workspaces = self.get_recent_workspaces(1).await;
                if let Some(workspace) = recent_workspaces.first() {
                    self.set_active_workspace(workspace.id.clone()).await?;
                    tracing::info!("Restored active workspace: {:?}", workspace.id);
                }
            }
        }

        tracing::info!("Session restoration completed");
        Ok(())
    }

    /// Switch to a different workspace
    pub async fn switch_workspace(&self, id: WorkspaceId) -> Result<(), CoreError> {
        // Save current workspace state before switching
        self.save_workspace_state().await?;
        
        // Set new active workspace
        self.set_active_workspace(id.clone()).await?;
        
        tracing::info!("Switched to workspace: {:?}", id);
        Ok(())
    }

    /// Get workspace configuration
    pub async fn get_config(&self) -> crate::core::config::WorkspaceConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// Update workspace configuration
    pub async fn update_config(&self, config: crate::core::config::WorkspaceConfig) -> Result<(), CoreError> {
        {
            let mut workspace_config = self.config.write().await;
            *workspace_config = config;
        }
        
        tracing::info!("Updated workspace configuration");
        Ok(())
    }
}

impl Default for WorkspaceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global workspace manager wrapper
#[derive(Clone)]
pub struct GlobalWorkspaceManager(pub Arc<WorkspaceManager>);

impl Global for GlobalWorkspaceManager {}

impl GlobalWorkspaceManager {
    /// Initialize the global workspace manager
    pub fn init(cx: &mut App) {
        let manager = Arc::new(WorkspaceManager::new());
        cx.set_global(GlobalWorkspaceManager(manager));
        tracing::info!("Global workspace manager initialized");
    }

    /// Get the global workspace manager
    pub fn get(cx: &App) -> Arc<WorkspaceManager> {
        cx.global::<GlobalWorkspaceManager>().0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_workspace_manager_creation() {
        let manager = WorkspaceManager::new();
        
        // Should start with no workspaces
        let workspaces = manager.list_workspaces().await;
        assert!(workspaces.is_empty());
        
        // Should have no active workspace
        let active = manager.get_active_workspace().await;
        assert!(active.is_none());
    }

    #[tokio::test]
    async fn test_create_workspace() {
        let manager = WorkspaceManager::new();
        
        let workspace_id = manager
            .create_workspace("Test Workspace".to_string(), None)
            .await
            .unwrap();
        
        let workspace = manager.get_workspace(&workspace_id).await.unwrap();
        assert_eq!(workspace.name, "Test Workspace");
        assert_eq!(workspace.id, workspace_id);
        assert!(workspace.path.is_none());
    }

    #[tokio::test]
    async fn test_set_active_workspace() {
        let manager = WorkspaceManager::new();
        
        let workspace_id = manager
            .create_workspace("Test Workspace".to_string(), None)
            .await
            .unwrap();
        
        manager.set_active_workspace(workspace_id.clone()).await.unwrap();
        
        let active = manager.get_active_workspace().await.unwrap();
        assert_eq!(active.id, workspace_id);
    }

    #[tokio::test]
    async fn test_workspace_with_path() {
        let manager = WorkspaceManager::new();
        let path = PathBuf::from("/test/path");
        
        let workspace_id = manager
            .create_workspace("Path Workspace".to_string(), Some(path.clone()))
            .await
            .unwrap();
        
        let workspace = manager.get_workspace(&workspace_id).await.unwrap();
        assert_eq!(workspace.path, Some(path));
    }

    #[tokio::test]
    async fn test_update_workspace_layout() {
        let manager = WorkspaceManager::new();
        
        let workspace_id = manager
            .create_workspace("Test Workspace".to_string(), None)
            .await
            .unwrap();
        
        let mut new_layout = WorkspaceLayout::default();
        new_layout.layout_type = "custom".to_string();
        
        manager
            .update_workspace_layout(&workspace_id, new_layout.clone())
            .await
            .unwrap();
        
        let workspace = manager.get_workspace(&workspace_id).await.unwrap();
        assert_eq!(workspace.layout.layout_type, "custom");
    }

    #[tokio::test]
    async fn test_remove_workspace() {
        let manager = WorkspaceManager::new();
        
        let workspace_id = manager
            .create_workspace("Test Workspace".to_string(), None)
            .await
            .unwrap();
        
        manager.set_active_workspace(workspace_id.clone()).await.unwrap();
        manager.remove_workspace(&workspace_id).await.unwrap();
        
        let workspace = manager.get_workspace(&workspace_id).await;
        assert!(workspace.is_none());
        
        let active = manager.get_active_workspace().await;
        assert!(active.is_none());
    }

    #[tokio::test]
    async fn test_recent_workspaces() {
        let manager = WorkspaceManager::new();
        
        // Create multiple workspaces
        let id1 = manager.create_workspace("Workspace 1".to_string(), None).await.unwrap();
        let id2 = manager.create_workspace("Workspace 2".to_string(), None).await.unwrap();
        let id3 = manager.create_workspace("Workspace 3".to_string(), None).await.unwrap();
        
        // Access them in different order
        manager.set_active_workspace(id1).await.unwrap();
        manager.set_active_workspace(id3.clone()).await.unwrap();
        manager.set_active_workspace(id2).await.unwrap();
        
        let recent = manager.get_recent_workspaces(2).await;
        assert_eq!(recent.len(), 2);
        // Most recent should be id2, then id3
        assert_eq!(recent[0].name, "Workspace 2");
        assert_eq!(recent[1].name, "Workspace 3");
    }

    #[tokio::test]
    async fn test_workspace_initialization() {
        let manager = WorkspaceManager::new();
        let config = TotthoConfig::default();
        
        manager.initialize(&config).await.unwrap();
        
        // Should create a default workspace
        let workspaces = manager.list_workspaces().await;
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].name, "Default");
        
        // Should set it as active
        let active = manager.get_active_workspace().await.unwrap();
        assert_eq!(active.name, "Default");
    }

    #[tokio::test]
    async fn test_session_restoration() {
        let manager = WorkspaceManager::new();
        
        manager.restore_session().await.unwrap();
        
        // Should create a default workspace if none exist
        let workspaces = manager.list_workspaces().await;
        assert_eq!(workspaces.len(), 1);
        
        let active = manager.get_active_workspace().await.unwrap();
        assert_eq!(active.name, "Default");
    }
}