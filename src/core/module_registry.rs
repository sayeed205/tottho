//! Module registry system for managing application modules
//! 
//! Provides module registration, dependency resolution, and lifecycle management.

use std::collections::HashMap;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::core::{CoreError, Result};

/// Information about a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Module name
    pub name: String,
    /// Module version
    pub version: String,
    /// Module dependencies
    pub dependencies: Vec<String>,
    /// Module capabilities
    pub capabilities: Vec<String>,
}

/// Current state of a module
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleState {
    /// Module is registered but not initialized
    Registered,
    /// Module is currently initializing
    Initializing,
    /// Module is ready and operational
    Ready,
    /// Module failed to initialize
    Failed(String),
    /// Module has been shutdown
    Shutdown,
}

/// Trait that all application modules must implement
#[async_trait]
pub trait Module: Send + Sync {
    /// Get module information
    fn info(&self) -> ModuleInfo;
    
    /// Get module dependencies
    fn dependencies(&self) -> Vec<&'static str> {
        Vec::new()
    }
    
    /// Initialize the module
    async fn initialize(&mut self) -> Result<()>;
    
    /// Shutdown the module
    async fn shutdown(&mut self) -> Result<()>;
    
    /// Get module name (convenience method)
    fn name(&self) -> &str {
        "unknown"
    }
}

/// Registry entry for a module
struct ModuleEntry {
    module: Box<dyn Module>,
    state: ModuleState,
    info: ModuleInfo,
}

/// Dependency graph for module initialization ordering
#[derive(Debug, Default)]
pub struct DependencyGraph {
    dependencies: HashMap<String, Vec<String>>,
}

impl DependencyGraph {
    /// Add a module and its dependencies
    pub fn add_module(&mut self, name: String, dependencies: Vec<String>) {
        self.dependencies.insert(name, dependencies);
    }

    /// Get initialization order using topological sort
    pub fn get_initialization_order(&self) -> Result<Vec<String>> {
        let mut visited = HashMap::new();
        let mut temp_visited = HashMap::new();
        let mut result = Vec::new();

        for module_name in self.dependencies.keys() {
            if !visited.contains_key(module_name) {
                self.visit(module_name, &mut visited, &mut temp_visited, &mut result)?;
            }
        }

        result.reverse();
        Ok(result)
    }

    fn visit(
        &self,
        module_name: &str,
        visited: &mut HashMap<String, bool>,
        temp_visited: &mut HashMap<String, bool>,
        result: &mut Vec<String>,
    ) -> Result<()> {
        if temp_visited.contains_key(module_name) {
            return Err(CoreError::CircularDependency {
                module: module_name.to_string(),
            });
        }

        if visited.contains_key(module_name) {
            return Ok(());
        }

        temp_visited.insert(module_name.to_string(), true);

        if let Some(dependencies) = self.dependencies.get(module_name) {
            for dep in dependencies {
                self.visit(dep, visited, temp_visited, result)?;
            }
        }

        temp_visited.remove(module_name);
        visited.insert(module_name.to_string(), true);
        result.push(module_name.to_string());

        Ok(())
    }
}

/// Registry for managing application modules
pub struct ModuleRegistry {
    /// Registered modules
    modules: HashMap<String, ModuleEntry>,
    /// Dependency graph
    dependency_graph: DependencyGraph,
}

impl ModuleRegistry {
    /// Create a new module registry
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            dependency_graph: DependencyGraph::default(),
        }
    }

    /// Register a module
    pub fn register(&mut self, module: Box<dyn Module>) -> Result<()> {
        let info = module.info();
        let name = info.name.clone();
        
        // Check if module is already registered
        if self.modules.contains_key(&name) {
            return Err(CoreError::ModuleAlreadyRegistered { name });
        }

        // Add to dependency graph
        self.dependency_graph.add_module(
            name.clone(),
            module.dependencies().iter().map(|s| s.to_string()).collect(),
        );

        // Validate module interface
        self.validate_module(&*module)?;

        // Create registry entry
        let entry = ModuleEntry {
            info: info.clone(),
            module,
            state: ModuleState::Registered,
        };

        self.modules.insert(name.clone(), entry);
        
        tracing::info!("Module '{}' registered successfully", name);
        Ok(())
    }

    /// Initialize all modules in dependency order
    pub async fn initialize_all(&mut self) -> Result<()> {
        let initialization_order = self.dependency_graph.get_initialization_order()?;
        
        for module_name in initialization_order {
            if let Some(entry) = self.modules.get_mut(&module_name) {
                entry.state = ModuleState::Initializing;
                
                match entry.module.initialize().await {
                    Ok(()) => {
                        entry.state = ModuleState::Ready;
                        tracing::info!("Module '{}' initialized successfully", module_name);
                    }
                    Err(e) => {
                        let error_msg = format!("{}", e);
                        entry.state = ModuleState::Failed(error_msg.clone());
                        tracing::error!("Module '{}' failed to initialize: {}", module_name, error_msg);
                        return Err(CoreError::ModuleInitializationFailed {
                            module: module_name,
                            error: error_msg,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Shutdown all modules
    pub async fn shutdown_all(&self) -> Result<()> {
        for (name, entry) in &self.modules {
            if entry.state == ModuleState::Ready {
                tracing::info!("Shutting down module '{}'", name);
                // Note: We can't modify the module here due to borrowing rules
                // This would need to be refactored for actual shutdown implementation
            }
        }
        Ok(())
    }

    /// Get module by name
    pub fn get_module(&self, name: &str) -> Option<&ModuleEntry> {
        self.modules.get(name)
    }

    /// Get module state
    pub fn get_module_state(&self, name: &str) -> Option<&ModuleState> {
        self.modules.get(name).map(|entry| &entry.state)
    }

    /// Get all module names
    pub fn get_module_names(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }

    /// Get modules by state
    pub fn get_modules_by_state(&self, state: &ModuleState) -> Vec<String> {
        self.modules
            .iter()
            .filter(|(_, entry)| &entry.state == state)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Validate module interface compliance
    fn validate_module(&self, module: &dyn Module) -> Result<()> {
        let info = module.info();
        
        // Validate module name
        if info.name.is_empty() {
            return Err(CoreError::InvalidModuleInterface {
                reason: "Module name cannot be empty".to_string(),
            });
        }

        // Validate dependencies exist (for registered modules)
        for dep in module.dependencies() {
            if !self.modules.contains_key(dep) {
                tracing::warn!("Module '{}' depends on unregistered module '{}'", info.name, dep);
            }
        }

        Ok(())
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}