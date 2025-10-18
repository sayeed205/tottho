//! Event bus system for inter-module communication
//! 
//! Provides asynchronous event routing and handling between application modules.

use std::any::TypeId;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::core::{CoreError, Result};

/// Unique identifier for events
pub type EventId = String;

/// Base trait for all events in the system
pub trait Event: Send + Sync + std::fmt::Debug + 'static {
    /// Get the event type name
    fn event_type(&self) -> &'static str;
    /// Get unique event ID
    fn event_id(&self) -> EventId {
        cuid2::create_id()
    }
}

/// Core events that the system generates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreEvent {
    /// Module was registered
    ModuleRegistered { name: String },
    /// Module was initialized
    ModuleInitialized { name: String },
    /// Module failed to initialize
    ModuleFailed { name: String, error: String },
    /// Application state changed
    StateChanged { component: String },
    /// Error occurred in the system
    ErrorOccurred { source: String, error: String },
    /// Shutdown was requested
    ShutdownRequested,
}

impl Event for CoreEvent {
    fn event_type(&self) -> &'static str {
        match self {
            CoreEvent::ModuleRegistered { .. } => "core.module_registered",
            CoreEvent::ModuleInitialized { .. } => "core.module_initialized",
            CoreEvent::ModuleFailed { .. } => "core.module_failed",
            CoreEvent::StateChanged { .. } => "core.state_changed",
            CoreEvent::ErrorOccurred { .. } => "core.error_occurred",
            CoreEvent::ShutdownRequested => "core.shutdown_requested",
        }
    }

    fn event_id(&self) -> EventId {
        cuid2::create_id()
    }
}

/// Handler for processing events
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle an event
    async fn handle(&self, event: &dyn Event) -> Result<()>;
    /// Get the handler name for debugging
    fn handler_name(&self) -> &'static str;
}

/// Event bus for routing messages between modules
pub struct EventBus {
    /// Event handlers organized by event type
    handlers: Arc<RwLock<HashMap<TypeId, Vec<Arc<dyn EventHandler>>>>>,
    /// Message queue for async processing
    message_queue: Arc<Mutex<VecDeque<Box<dyn Event>>>>,
    /// Flag to indicate if the bus is running
    running: Arc<RwLock<bool>>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Initialize the event bus
    pub async fn initialize(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = true;
        tracing::debug!("Event bus initialized");
        Ok(())
    }

    /// Subscribe to events of a specific type
    pub async fn subscribe<T: Event>(&self, handler: Arc<dyn EventHandler>) -> Result<()> {
        let type_id = TypeId::of::<T>();
        let mut handlers = self.handlers.write().await;
        
        handlers.entry(type_id).or_insert_with(Vec::new).push(handler);
        
        tracing::debug!("Handler subscribed to event type: {:?}", type_id);
        Ok(())
    }

    /// Publish an event to all subscribers
    pub async fn publish<T: Event>(&self, event: T) -> Result<()> {
        let running = self.running.read().await;
        if !*running {
            return Err(CoreError::EventBusNotRunning);
        }

        let type_id = TypeId::of::<T>();
        let handlers = self.handlers.read().await;
        
        if let Some(event_handlers) = handlers.get(&type_id) {
            for handler in event_handlers {
                if let Err(e) = handler.handle(&event).await {
                    tracing::error!(
                        "Event handler '{}' failed to process event: {}",
                        handler.handler_name(),
                        e
                    );
                }
            }
        }

        tracing::trace!("Published event: {:?}", event.event_type());
        Ok(())
    }

    /// Process queued events (for future batch processing)
    pub async fn process_queue(&self) -> Result<()> {
        let mut queue = self.message_queue.lock().await;
        
        while let Some(event) = queue.pop_front() {
            // Process event through handlers
            // This is a placeholder for future batch processing implementation
            tracing::trace!("Processing queued event: {}", event.event_type());
        }
        
        Ok(())
    }

    /// Shutdown the event bus
    pub async fn shutdown(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        
        // Process any remaining queued events
        self.process_queue().await?;
        
        tracing::debug!("Event bus shutdown");
        Ok(())
    }

    /// Get the number of registered handlers
    pub async fn handler_count(&self) -> usize {
        let handlers = self.handlers.read().await;
        handlers.values().map(|v| v.len()).sum()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}