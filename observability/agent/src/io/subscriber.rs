//! subscriber.rs — CommandSubscriber for ai.command.execute
//!
//! Listens to incoming AI commands and forwards them to CommandExecutor.

use std::sync::Arc;

use crate::commands::CommandExecutor;
use crate::error::AgentError;
use rhelma_event::EventBus;

/// Command subscriber for AI command execution
pub struct CommandSubscriber<B: EventBus + Send + Sync + 'static> {
    /// Command executor instance
    executor: Arc<CommandExecutor<B>>,
}

impl<B> CommandSubscriber<B>
where
    B: EventBus + Send + Sync + 'static,
{
    /// Creates a new command subscriber
    ///
    /// # Arguments
    /// * `executor` - Command executor instance
    ///
    /// # Returns
    /// A new command subscriber
    pub fn new(executor: Arc<CommandExecutor<B>>) -> Self {
        Self { executor }
    }

    /// Handles incoming AI command
    ///
    /// # Arguments
    /// * `cmd` - AI command to execute
    ///
    /// # Returns
    /// `Result<(), AgentError>` - Success or error
    pub async fn handle_command(
        &self,
        cmd: rhelma_event::contracts::ai::AiCommandExecute,
    ) -> Result<(), AgentError> {
        self.executor.execute(cmd).await
    }
}
