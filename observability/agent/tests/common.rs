use async_trait::async_trait;
use rhelma_event::{EventBus, EventBusError, EventEnvelope};
use std::sync::{Arc, Mutex};

#[derive(Default, Clone)]
pub struct FakeEventBus {
    /// Field `published`.
    pub published: Arc<Mutex<Vec<EventEnvelope>>>,
}

#[async_trait]
impl EventBus for FakeEventBus {
    async fn publish(&self, env: EventEnvelope) -> Result<(), EventBusError> {
        self.published.lock().unwrap().push(env);
        Ok(())
    }
}
