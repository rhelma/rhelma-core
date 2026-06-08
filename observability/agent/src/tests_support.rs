use async_trait::async_trait;
use rhelma_event::{EventBus, EventEnvelope, EventBusError};
use std::sync::{Arc, Mutex};

/// A minimal but test-friendly EventBus implementation.
///
/// - Stores all published envelopes
/// - Allows inspection in tests (last(), count(), clear(), etc.)
/// - Thread-safe & poison-safe
#[derive(Clone, Default)]
pub struct FakeEventBus {
    /// Field `published`.
    pub published: Arc<Mutex<Vec<EventEnvelope>>>,
}

impl FakeEventBus {
    /// Return number of published events.
    pub fn count(&self) -> usize {
        self.published.lock().map(|v| v.len()).unwrap_or(0)
    }

    /// Return last published event (if any).
    pub fn last(&self) -> Option<EventEnvelope> {
        self.published
            .lock()
            .ok()
            .and_then(|v| v.last().cloned())
    }

    /// Return all events.
    pub fn all(&self) -> Vec<EventEnvelope> {
        self.published
            .lock()
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Remove all events.
    pub fn clear(&self) {
        if let Ok(mut v) = self.published.lock() {
            v.clear();
        }
    }

    /// Pop last event (useful in tests).
    pub fn pop(&self) -> Option<EventEnvelope> {
        self.published
            .lock()
            .ok()
            .and_then(|mut v| v.pop())
    }
}

#[async_trait]
impl EventBus for FakeEventBus {
    async fn publish(&self, env: EventEnvelope) -> Result<(), EventBusError> {
        // Never panic in test bus
        if let Ok(mut v) = self.published.lock() {
            v.push(env);
        }
        Ok(())
    }
}
