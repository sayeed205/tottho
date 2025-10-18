//! Module registry system for managing application modules
//! 
//! Provides module registration, dependency resolution, and lifecycle management.

use std::collections::HashMap;
use std::any::Any;
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
    /// Get module name
    fn name(&self) -> &'static str;
    
    /// Get module dependencies
    fn dependencies(&self) -> Vec<&'static str>;
    
    /// Initialize the module
    async fn initialize(&mut self) -> Result<()>;
    
    /// Shutdown the module
    async fn shutdown(&mut self) -> Result<()>;
    
    /// Support for downcasting to concrete types
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Registry entry for a module
struct ModuleEntry {
    module: Box<dyn Module>,
    state: ModuleState,
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

    /// Remove a module from the dependency graph
    pub fn remove_module(&mut self, name: &str) {
        self.dependencies.remove(name);
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
    pub fn register<T: Module + 'static>(&mut self, module: T) -> Result<()> {
        let name = module.name().to_string();
        
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
        self.validate_module(&module)?;

        // Create registry entry
        let entry = ModuleEntry {
            module: Box::new(module),
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

    /// Shutdown all modules in reverse dependency order
    pub async fn shutdown_all(&mut self) -> Result<()> {
        let shutdown_order = self.get_shutdown_order()?;
        
        for module_name in shutdown_order {
            if let Some(entry) = self.modules.get_mut(&module_name) {
                if entry.state == ModuleState::Ready {
                    tracing::info!("Shutting down module '{}'", module_name);
                    match entry.module.shutdown().await {
                        Ok(()) => {
                            entry.state = ModuleState::Shutdown;
                            tracing::info!("Module '{}' shutdown successfully", module_name);
                        }
                        Err(e) => {
                            tracing::error!("Module '{}' failed to shutdown: {}", module_name, e);
                            // Continue shutting down other modules even if one fails
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Get shutdown order (reverse of initialization order)
    fn get_shutdown_order(&self) -> Result<Vec<String>> {
        let mut init_order = self.dependency_graph.get_initialization_order()?;
        init_order.reverse();
        Ok(init_order)
    }

    /// Get module by name
    pub fn get_module(&self, name: &str) -> Option<&ModuleEntry> {
        self.modules.get(name)
    }

    /// Get module by type (downcast from trait object)
    pub fn get_module_typed<T: Module + 'static>(&self, name: &str) -> Option<&T> {
        self.modules.get(name)
            .and_then(|entry| entry.module.as_ref().as_any().downcast_ref::<T>())
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

    /// Unregister a module (for dynamic loading/unloading)
    pub async fn unregister(&mut self, module_name: &str) -> Result<()> {
        // Check if module exists
        if !self.modules.contains_key(module_name) {
            return Err(CoreError::ModuleNotFound { 
                module: module_name.to_string() 
            });
        }

        // Check if any other modules depend on this one
        let dependents = self.get_dependents(module_name);
        if !dependents.is_empty() {
            return Err(CoreError::ModuleHasDependents {
                module: module_name.to_string(),
                dependents,
            });
        }

        // Get the module entry and shutdown if needed
        let mut entry = self.modules.remove(module_name).unwrap();
        if entry.state == ModuleState::Ready {
            tracing::info!("Shutting down module '{}' before unregistering", module_name);
            entry.module.shutdown().await?;
        }

        // Remove from dependency graph
        self.dependency_graph.remove_module(module_name);
        
        tracing::info!("Module '{}' unregistered successfully", module_name);
        Ok(())
    }

    /// Get modules that depend on the given module
    fn get_dependents(&self, module_name: &str) -> Vec<String> {
        self.modules
            .iter()
            .filter(|(_, entry)| {
                entry.module.dependencies().contains(&module_name)
            })
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Initialize a specific module (for dynamic loading)
    pub async fn initialize_module(&mut self, module_name: &str) -> Result<()> {
        // Check if module exists
        if !self.modules.contains_key(module_name) {
            return Err(CoreError::ModuleNotFound { 
                module: module_name.to_string() 
            });
        }

        // Check if dependencies are satisfied
        let dependencies: Vec<&'static str> = {
            let entry = self.modules.get(module_name).unwrap();
            entry.module.dependencies()
        };

        for dep in &dependencies {
            let dep_state = self.get_module_state(dep);
            if dep_state != Some(&ModuleState::Ready) {
                return Err(CoreError::ModuleDependencyNotReady {
                    module: module_name.to_string(),
                    dependency: dep.to_string(),
                });
            }
        }

        // Initialize the module
        let entry = self.modules.get_mut(module_name).unwrap();
        entry.state = ModuleState::Initializing;
        match entry.module.initialize().await {
            Ok(()) => {
                entry.state = ModuleState::Ready;
                tracing::info!("Module '{}' initialized successfully", module_name);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                entry.state = ModuleState::Failed(error_msg.clone());
                tracing::error!("Module '{}' failed to initialize: {}", module_name, error_msg);
                Err(CoreError::ModuleInitializationFailed {
                    module: module_name.to_string(),
                    error: error_msg,
                })
            }
        }
    }

    /// Validate module interface compliance
    fn validate_module(&self, module: &dyn Module) -> Result<()> {
        let name = module.name();
        
        // Validate module name
        if name.is_empty() {
            return Err(CoreError::InvalidModuleInterface {
                reason: "Module name cannot be empty".to_string(),
            });
        }

        // Validate dependencies exist (for registered modules)
        for dep in module.dependencies() {
            if !self.modules.contains_key(dep) {
                tracing::warn!("Module '{}' depends on unregistered module '{}'", name, dep);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // Test module implementation
    struct TestModule {
        name: &'static str,
        dependencies: Vec<&'static str>,
        initialized: Arc<AtomicBool>,
        should_fail: bool,
    }

    impl TestModule {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                dependencies: Vec::new(),
                initialized: Arc::new(AtomicBool::new(false)),
                should_fail: false,
            }
        }

        fn with_dependencies(name: &'static str, deps: Vec<&'static str>) -> Self {
            Self {
                name,
                dependencies: deps,
                initialized: Arc::new(AtomicBool::new(false)),
                should_fail: false,
            }
        }

        fn with_failure(name: &'static str) -> Self {
            Self {
                name,
                dependencies: Vec::new(),
                initialized: Arc::new(AtomicBool::new(false)),
                should_fail: true,
            }
        }

        fn is_initialized(&self) -> bool {
            self.initialized.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl Module for TestModule {
        fn name(&self) -> &'static str {
            self.name
        }

        fn dependencies(&self) -> Vec<&'static str> {
            self.dependencies.clone()
        }

        async fn initialize(&mut self) -> Result<()> {
            if self.should_fail {
                return Err(CoreError::Internal {
                    reason: "Test failure".to_string(),
                });
            }
            self.initialized.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<()> {
            self.initialized.store(false, Ordering::SeqCst);
            Ok(())
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[tokio::test]
    async fn test_module_registration() {
        let mut registry = ModuleRegistry::new();
        let module = TestModule::new("test_module");

        // Test successful registration
        assert!(registry.register(module).is_ok());
        assert!(registry.modules.contains_key("test_module"));
        assert_eq!(registry.get_module_names().len(), 1);
    }

    #[tokio::test]
    async fn test_duplicate_module_registration() {
        let mut registry = ModuleRegistry::new();
        let module1 = TestModule::new("test_module");
        let module2 = TestModule::new("test_module");

        // Register first module
        assert!(registry.register(module1).is_ok());

        // Try to register duplicate
        let result = registry.register(module2);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CoreError::ModuleAlreadyRegistered { .. }));
    }

    #[tokio::test]
    async fn test_module_initialization() {
        let mut registry = ModuleRegistry::new();
        let module = TestModule::new("test_module");
        let initialized_flag = module.initialized.clone();

        registry.register(module).unwrap();

        // Test initialization
        assert!(registry.initialize_all().await.is_ok());
        assert!(initialized_flag.load(Ordering::SeqCst));
        
        // Check module state
        assert_eq!(
            registry.get_module_state("test_module"),
            Some(&ModuleState::Ready)
        );
    }

    #[tokio::test]
    async fn test_module_initialization_failure() {
        let mut registry = ModuleRegistry::new();
        let module = TestModule::with_failure("failing_module");

        registry.register(module).unwrap();

        // Test initialization failure
        let result = registry.initialize_all().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CoreError::ModuleInitializationFailed { .. }));
        
        // Check module state
        assert!(matches!(
            registry.get_module_state("failing_module"),
            Some(ModuleState::Failed(_))
        ));
    }

    #[tokio::test]
    async fn test_dependency_resolution() {
        let mut registry = ModuleRegistry::new();
        
        // Create modules with dependencies
        let module_a = TestModule::new("module_a");
        let module_b = TestModule::with_dependencies("module_b", vec!["module_a"]);
        let module_c = TestModule::with_dependencies("module_c", vec!["module_b"]);

        // Register in reverse order to test dependency resolution
        registry.register(module_c).unwrap();
        registry.register(module_b).unwrap();
        registry.register(module_a).unwrap();

        // Test initialization order
        assert!(registry.initialize_all().await.is_ok());
        
        // All modules should be ready
        assert_eq!(registry.get_modules_by_state(&ModuleState::Ready).len(), 3);
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let mut registry = ModuleRegistry::new();
        
        // Create modules with circular dependencies
        let module_a = TestModule::with_dependencies("module_a", vec!["module_b"]);
        let module_b = TestModule::with_dependencies("module_b", vec!["module_a"]);

        registry.register(module_a).unwrap();
        registry.register(module_b).unwrap();

        // Test circular dependency detection
        let result = registry.initialize_all().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CoreError::CircularDependency { .. }));
    }

    #[tokio::test]
    async fn test_module_validation() {
        let mut registry = ModuleRegistry::new();
        
        // Test empty name validation
        struct EmptyNameModule;
        
        #[async_trait]
        impl Module for EmptyNameModule {
            fn name(&self) -> &'static str { "" }
            fn dependencies(&self) -> Vec<&'static str> { Vec::new() }
            async fn initialize(&mut self) -> Result<()> { Ok(()) }
            async fn shutdown(&mut self) -> Result<()> { Ok(()) }
            fn as_any(&self) -> &dyn Any { self }
        }

        let result = registry.register(EmptyNameModule);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CoreError::InvalidModuleInterface { .. }));
    }

    #[tokio::test]
    async fn test_get_module_typed() {
        let mut registry = ModuleRegistry::new();
        let module = TestModule::new("test_module");

        registry.register(module).unwrap();

        // Test typed module retrieval
        let retrieved = registry.get_module_typed::<TestModule>("test_module");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test_module");

        // Test non-existent module
        let non_existent = registry.get_module_typed::<TestModule>("non_existent");
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::default();
        
        // Add modules with dependencies
        graph.add_module("a".to_string(), vec![]);
        graph.add_module("b".to_string(), vec!["a".to_string()]);
        graph.add_module("c".to_string(), vec!["b".to_string()]);

        // Test initialization order
        let order = graph.get_initialization_order().unwrap();
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_dependency_graph_circular() {
        let mut graph = DependencyGraph::default();
        
        // Add modules with circular dependencies
        graph.add_module("a".to_string(), vec!["b".to_string()]);
        graph.add_module("b".to_string(), vec!["a".to_string()]);

        // Test circular dependency detection
        let result = graph.get_initialization_order();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CoreError::CircularDependency { .. }));
    }

    #[test]
    fn test_module_states() {
        let registry = ModuleRegistry::new();
        
        // Test getting modules by state
        let ready_modules = registry.get_modules_by_state(&ModuleState::Ready);
        assert!(ready_modules.is_empty());
        
        let registered_modules = registry.get_modules_by_state(&ModuleState::Registered);
        assert!(registered_modules.is_empty());
    }

    #[tokio::test]
    async fn test_dynamic_module_initialization() {
        let mut registry = ModuleRegistry::new();
        let module_a = TestModule::new("module_a");
        let module_b = TestModule::with_dependencies("module_b", vec!["module_a"]);

        registry.register(module_a).unwrap();
        registry.register(module_b).unwrap();

        // Initialize module_a first
        assert!(registry.initialize_module("module_a").await.is_ok());
        assert_eq!(registry.get_module_state("module_a"), Some(&ModuleState::Ready));

        // Now initialize module_b (which depends on module_a)
        assert!(registry.initialize_module("module_b").await.is_ok());
        assert_eq!(registry.get_module_state("module_b"), Some(&ModuleState::Ready));
    }

    #[tokio::test]
    async fn test_dynamic_module_initialization_missing_dependency() {
        let mut registry = ModuleRegistry::new();
        let module_b = TestModule::with_dependencies("module_b", vec!["module_a"]);

        registry.register(module_b).unwrap();

        // Try to initialize module_b without module_a being ready
        let result = registry.initialize_module("module_b").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CoreError::ModuleDependencyNotReady { .. }));
    }

    #[tokio::test]
    async fn test_module_unregistration() {
        let mut registry = ModuleRegistry::new();
        let module = TestModule::new("test_module");

        registry.register(module).unwrap();
        registry.initialize_module("test_module").await.unwrap();

        // Test unregistration
        assert!(registry.unregister("test_module").await.is_ok());
        assert!(!registry.modules.contains_key("test_module"));
    }

    #[tokio::test]
    async fn test_module_unregistration_with_dependents() {
        let mut registry = ModuleRegistry::new();
        let module_a = TestModule::new("module_a");
        let module_b = TestModule::with_dependencies("module_b", vec!["module_a"]);

        registry.register(module_a).unwrap();
        registry.register(module_b).unwrap();

        // Try to unregister module_a while module_b depends on it
        let result = registry.unregister("module_a").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CoreError::ModuleHasDependents { .. }));
    }

    #[tokio::test]
    async fn test_shutdown_order() {
        let mut registry = ModuleRegistry::new();
        let module_a = TestModule::new("module_a");
        let module_b = TestModule::with_dependencies("module_b", vec!["module_a"]);
        let module_c = TestModule::with_dependencies("module_c", vec!["module_b"]);

        let a_flag = module_a.initialized.clone();
        let b_flag = module_b.initialized.clone();
        let c_flag = module_c.initialized.clone();

        registry.register(module_a).unwrap();
        registry.register(module_b).unwrap();
        registry.register(module_c).unwrap();

        // Initialize all modules
        registry.initialize_all().await.unwrap();
        assert!(a_flag.load(Ordering::SeqCst));
        assert!(b_flag.load(Ordering::SeqCst));
        assert!(c_flag.load(Ordering::SeqCst));

        // Shutdown all modules
        registry.shutdown_all().await.unwrap();
        
        // All modules should be shutdown
        assert_eq!(registry.get_modules_by_state(&ModuleState::Shutdown).len(), 3);
    }

    #[test]
    fn test_dependency_graph_remove_module() {
        let mut graph = DependencyGraph::default();
        
        graph.add_module("a".to_string(), vec![]);
        graph.add_module("b".to_string(), vec!["a".to_string()]);
        
        assert!(graph.dependencies.contains_key("a"));
        assert!(graph.dependencies.contains_key("b"));
        
        graph.remove_module("b");
        assert!(!graph.dependencies.contains_key("b"));
        assert!(graph.dependencies.contains_key("a"));
    }
}