//! Component Registry
//! 
//! Manages UI component registration, lifecycle, and dependency tracking
//! for extensible UI system.

use std::collections::HashMap;
use std::any::TypeId;
use crate::ui::{UIError, UIResult};

/// Component registry for UI system
pub struct ComponentRegistry {
    components: HashMap<TypeId, ComponentDescriptor>,
    component_states: HashMap<TypeId, ComponentState>,
}

/// UI component trait
pub trait UIComponent: Send + Sync + 'static {
    /// Get component name
    fn name(&self) -> &'static str;
    
    /// Initialize the component
    fn initialize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
    
    /// Shutdown the component
    fn shutdown(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
    
    /// Get component dependencies
    fn dependencies(&self) -> Vec<TypeId> {
        Vec::new()
    }
}

/// Component descriptor for registry
struct ComponentDescriptor {
    name: String,
    type_id: TypeId,
}

/// Component lifecycle state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentState {
    Registered,
    Initializing,
    Active,
    Failed(String),
    Shutdown,
}

impl ComponentRegistry {
    /// Create a new component registry
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            component_states: HashMap::new(),
        }
    }
    
    /// Register a UI component
    pub fn register_component<T: UIComponent>(&mut self, component: T) -> anyhow::Result<()> {
        let type_id = TypeId::of::<T>();
        let name = component.name().to_string();
        
        if self.components.contains_key(&type_id) {
            return Err(anyhow::anyhow!("Component '{}' already registered", name));
        }
        
        let descriptor = ComponentDescriptor {
            name,
            type_id,
        };
        
        self.components.insert(type_id, descriptor);
        self.component_states.insert(type_id, ComponentState::Registered);
        
        Ok(())
    }
    
    /// Get component state
    pub fn get_component_state<T: UIComponent>(&self) -> Option<&ComponentState> {
        let type_id = TypeId::of::<T>();
        self.component_states.get(&type_id)
    }
    
    /// Set component state
    pub fn set_component_state<T: UIComponent>(&mut self, state: ComponentState) {
        let type_id = TypeId::of::<T>();
        self.component_states.insert(type_id, state);
    }
    
    /// Check if component is registered
    pub fn is_registered<T: UIComponent>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        self.components.contains_key(&type_id)
    }
    
    /// Get all registered component names
    pub fn get_component_names(&self) -> Vec<&str> {
        self.components.values()
            .map(|desc| desc.name.as_str())
            .collect()
    }
    
    /// Get component states summary
    pub fn get_states_summary(&self) -> HashMap<String, ComponentState> {
        let mut summary = HashMap::new();
        
        for (type_id, descriptor) in &self.components {
            if let Some(state) = self.component_states.get(type_id) {
                summary.insert(descriptor.name.clone(), state.clone());
            }
        }
        
        summary
    }
}