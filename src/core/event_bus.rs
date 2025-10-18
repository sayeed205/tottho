//! Event bus system for inter-module communication
//!
//! Provides asynchronous event routing and handling between application modules.
//! Implements HashMap-based channel storage for efficient event routing.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, trace, warn};

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
    /// Get the TypeId for this event (used internally for routing)
    fn type_id(&self) -> TypeId;
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

    fn type_id(&self) -> TypeId {
        TypeId::of::<CoreEvent>()
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
///
/// Uses HashMap-based channel storage for efficient event routing and supports
/// asynchronous message processing with batching capabilities.
pub struct EventBus {
    /// Event handlers organized by event type (HashMap-based channel storage)
    handlers: Arc<RwLock<HashMap<TypeId, Vec<Arc<dyn EventHandler>>>>>,
    /// Message queue for async processing and batching
    message_queue: Arc<Mutex<VecDeque<QueuedEvent>>>,
    /// Flag to indicate if the bus is running
    running: Arc<RwLock<bool>>,
    /// Statistics for monitoring
    stats: Arc<RwLock<EventBusStats>>,
}

/// Queued event with metadata for batch processing
#[derive(Debug)]
struct QueuedEvent {
    event: Box<dyn Event>,
    timestamp: std::time::Instant,
    retry_count: u32,
}

/// Event bus statistics for monitoring
#[derive(Debug, Default, Clone)]
pub struct EventBusStats {
    pub events_published: u64,
    pub events_processed: u64,
    pub handler_errors: u64,
    pub queue_size: usize,
}

impl EventBus {
    /// Create a new event bus with HashMap-based channel storage
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(EventBusStats::default())),
        }
    }

    /// Initialize the event bus
    pub async fn initialize(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = true;
        tracing::debug!("Event bus initialized");
        Ok(())
    }

    /// Subscribe to events of a specific type using HashMap-based channel storage
    pub async fn subscribe<T: Event>(&self, handler: Arc<dyn EventHandler>) -> Result<()> {
        let type_id = TypeId::of::<T>();
        let mut handlers = self.handlers.write().await;

        // Use HashMap-based storage for efficient event routing
        handlers
            .entry(type_id)
            .or_insert_with(Vec::new)
            .push(handler.clone());

        debug!(
            "Handler '{}' subscribed to event type: {:?}",
            handler.handler_name(),
            type_id
        );
        Ok(())
    }

    /// Publish an event to all subscribers using HashMap-based routing
    pub async fn publish<T: Event>(&self, event: T) -> Result<()> {
        let running = self.running.read().await;
        if !*running {
            return Err(CoreError::EventBusNotRunning);
        }
        drop(running);

        let type_id = TypeId::of::<T>();

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.events_published += 1;
        }

        // Get handlers and release the lock before calling them
        let event_handlers = {
            let handlers = self.handlers.read().await;
            handlers.get(&type_id).cloned()
        };

        // Route event using HashMap-based channel storage
        if let Some(event_handlers) = event_handlers {
            let mut error_count = 0;

            for handler in &event_handlers {
                if let Err(e) = handler.handle(&event).await {
                    error_count += 1;
                    error!(
                        "Event handler '{}' failed to process event '{}': {}",
                        handler.handler_name(),
                        event.event_type(),
                        e
                    );

                    // Update error statistics
                    let mut stats = self.stats.write().await;
                    stats.handler_errors += 1;
                }
            }

            if error_count > 0 {
                warn!(
                    "Event '{}' had {} handler errors out of {} handlers",
                    event.event_type(),
                    error_count,
                    event_handlers.len()
                );
            }
        }

        trace!("Published event: {}", event.event_type());
        Ok(())
    }

    /// Add event to queue for batch processing
    pub async fn queue_event<T: Event>(&self, event: T) -> Result<()> {
        let running = self.running.read().await;
        if !*running {
            return Err(CoreError::EventBusNotRunning);
        }

        let queued_event = QueuedEvent {
            event: Box::new(event),
            timestamp: std::time::Instant::now(),
            retry_count: 0,
        };

        let mut queue = self.message_queue.lock().await;
        queue.push_back(queued_event);

        // Update queue size statistics
        {
            let mut stats = self.stats.write().await;
            stats.queue_size = queue.len();
        }

        trace!("Event queued for batch processing");
        Ok(())
    }

    /// Process queued events with VecDeque for event batching
    pub async fn process_queue(&self) -> Result<()> {
        let running = self.running.read().await;
        if !*running {
            return Err(CoreError::EventBusNotRunning);
        }
        drop(running);

        let mut processed_count = 0;
        let mut error_count = 0;

        loop {
            let queued_event = {
                let mut queue = self.message_queue.lock().await;
                queue.pop_front()
            };

            let Some(mut queued_event) = queued_event else {
                break;
            };

            // Process event through handlers with error handling
            match self.process_queued_event(&queued_event.event).await {
                Ok(()) => {
                    processed_count += 1;
                    trace!(
                        "Processed queued event: {}",
                        queued_event.event.event_type()
                    );
                }
                Err(e) => {
                    error_count += 1;
                    queued_event.retry_count += 1;

                    error!(
                        "Failed to process queued event '{}' (attempt {}): {}",
                        queued_event.event.event_type(),
                        queued_event.retry_count,
                        e
                    );

                    // Retry logic - requeue if under retry limit
                    if queued_event.retry_count < 3 {
                        let mut queue = self.message_queue.lock().await;
                        queue.push_back(queued_event);
                        warn!("Requeued event for retry");
                    } else {
                        error!(
                            "Event '{}' exceeded retry limit, dropping",
                            queued_event.event.event_type()
                        );
                    }
                }
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.events_processed += processed_count;
            stats.handler_errors += error_count;
            stats.queue_size = 0; // Queue is now empty
        }

        if processed_count > 0 {
            debug!(
                "Processed {} queued events ({} errors)",
                processed_count, error_count
            );
        }

        Ok(())
    }

    /// Process a single queued event through all handlers
    async fn process_queued_event(&self, event: &Box<dyn Event>) -> Result<()> {
        let type_id = event.as_ref().type_id();

        // Get handlers and release the lock before calling them
        let event_handlers = {
            let handlers = self.handlers.read().await;
            handlers.get(&type_id).cloned()
        };

        if let Some(event_handlers) = event_handlers {
            for handler in &event_handlers {
                if let Err(e) = handler.handle(event.as_ref()).await {
                    return Err(CoreError::EventHandlerError {
                        handler: handler.handler_name().to_string(),
                        error: e.to_string(),
                    });
                }
            }
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

    /// Get current queue size
    pub async fn queue_size(&self) -> usize {
        let queue = self.message_queue.lock().await;
        queue.len()
    }

    /// Get event bus statistics
    pub async fn get_stats(&self) -> EventBusStats {
        let stats = self.stats.read().await;
        EventBusStats {
            events_published: stats.events_published,
            events_processed: stats.events_processed,
            handler_errors: stats.handler_errors,
            queue_size: stats.queue_size,
        }
    }

    /// Clear all statistics
    pub async fn clear_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = EventBusStats::default();
    }

    /// Check if the event bus is running
    pub async fn is_running(&self) -> bool {
        let running = self.running.read().await;
        *running
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    // Test event types
    #[derive(Debug, Clone)]
    struct TestEvent {
        pub message: String,
        pub id: u32,
    }

    impl Event for TestEvent {
        fn event_type(&self) -> &'static str {
            "test.event"
        }

        fn type_id(&self) -> TypeId {
            TypeId::of::<TestEvent>()
        }
    }

    #[derive(Debug, Clone)]
    struct AnotherTestEvent {
        pub data: String,
    }

    impl Event for AnotherTestEvent {
        fn event_type(&self) -> &'static str {
            "test.another_event"
        }

        fn type_id(&self) -> TypeId {
            TypeId::of::<AnotherTestEvent>()
        }
    }

    // Simple test event handler that counts events without async operations
    struct SimpleHandler {
        name: &'static str,
        count: Arc<AtomicU32>,
        should_error: bool,
    }

    impl SimpleHandler {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                count: Arc::new(AtomicU32::new(0)),
                should_error: false,
            }
        }

        fn new_with_error(name: &'static str) -> Self {
            Self {
                name,
                count: Arc::new(AtomicU32::new(0)),
                should_error: true,
            }
        }

        fn get_count(&self) -> u32 {
            self.count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl EventHandler for SimpleHandler {
        async fn handle(&self, _event: &dyn Event) -> Result<()> {
            if self.should_error {
                return Err(CoreError::Internal {
                    reason: format!("Handler {} intentionally failed", self.name),
                });
            }

            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn handler_name(&self) -> &'static str {
            self.name
        }
    }

    #[tokio::test]
    async fn test_event_bus_not_running_errors() {
        let event_bus = EventBus::new();
        // Don't initialize the event bus

        // Publishing should fail when not running
        let test_event = TestEvent {
            message: "Should fail".to_string(),
            id: 1,
        };
        let result = event_bus.publish(test_event).await;
        assert!(matches!(result, Err(CoreError::EventBusNotRunning)));

        // Queueing should fail when not running
        let test_event2 = TestEvent {
            message: "Should also fail".to_string(),
            id: 2,
        };
        let result = event_bus.queue_event(test_event2).await;
        assert!(matches!(result, Err(CoreError::EventBusNotRunning)));

        // Processing queue should fail when not running
        let result = event_bus.process_queue().await;
        assert!(matches!(result, Err(CoreError::EventBusNotRunning)));
    }

    #[test]
    fn test_event_bus_creation() {
        let event_bus = EventBus::new();

        // Test that event bus can be created
        assert_eq!(
            std::mem::size_of_val(&event_bus),
            std::mem::size_of::<EventBus>()
        );
    }

    #[test]
    fn test_core_event_types() {
        // Test that core events implement the Event trait correctly
        let event = CoreEvent::ModuleRegistered {
            name: "test".to_string(),
        };
        assert_eq!(event.event_type(), "core.module_registered");

        let event2 = CoreEvent::ShutdownRequested;
        assert_eq!(event2.event_type(), "core.shutdown_requested");

        let event3 = CoreEvent::ErrorOccurred {
            source: "test".to_string(),
            error: "error".to_string(),
        };
        assert_eq!(event3.event_type(), "core.error_occurred");
    }

    #[test]
    fn test_event_trait_implementation() {
        let test_event = TestEvent {
            message: "test".to_string(),
            id: 1,
        };

        assert_eq!(test_event.event_type(), "test.event");
        assert_eq!(test_event.type_id(), TypeId::of::<TestEvent>());

        let another_event = AnotherTestEvent {
            data: "test".to_string(),
        };

        assert_eq!(another_event.event_type(), "test.another_event");
        assert_eq!(another_event.type_id(), TypeId::of::<AnotherTestEvent>());
    }
}
