//! nats_source.rs — Optional NATS adapter for ai.command.execute
//!
//! NOTE: This is a basic implementation. Kafka is still the primary transport.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use async_nats::Client;

use rhelma_tracing::context;

use rhelma_event::contracts::ai::AiCommandExecute;
use rhelma_event::EventBus;

use crate::commands::CommandExecutor;
use crate::error::AgentError;
use crate::runtime::ExternalCommandSource;

/// NATS-based command source.
pub struct NatsCommandSource<B: EventBus + Send + Sync + 'static> {
    client: Client,
    subject: String,
    _marker: std::marker::PhantomData<B>,
}

impl<B> NatsCommandSource<B>
where
    B: EventBus + Send + Sync + 'static,
{
    /// Connect to a NATS server and prepare to consume ai.command.execute messages.
    pub async fn connect(url: &str, subject: &str) -> Result<Self, AgentError> {
        let client = async_nats::connect(url)
            .await
            .map_err(|e| AgentError::internal(format!("failed to connect NATS: {e}")))?;

        Ok(Self {
            client,
            subject: subject.into(),
            _marker: std::marker::PhantomData,
        })
    }
}

impl<B> ExternalCommandSource<B> for NatsCommandSource<B>
where
    B: EventBus + Send + Sync + 'static,
{
    fn start(
        self: Arc<Self>,
        executor: Arc<CommandExecutor<B>>,
        shutdown: CancellationToken,
    ) -> JoinHandle<()> {
        let client = self.client.clone();
        let subject = self.subject.clone();

        tokio::spawn(async move {
            let mut sub = match client.subscribe(subject.clone()).await {
                Ok(s) => s,
                Err(e) => {
                    error!("[agent] NATS subscribe failed: {e}");
                    return;
                }
            };

            info!("[agent] NatsCommandSource listening on subject={subject}");

            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => {
                        info!("[agent] NatsCommandSource shutdown requested; stopping");
                        break;
                    }
                    maybe = sub.next() => {
                        let Some(msg) = maybe else { break; };

                        match serde_json::from_slice::<AiCommandExecute>(&msg.payload) {
                            Ok(cmd) => {
                                // Best-effort: scope tracing context from NATS headers (if present).
                                let mut h: HashMap<String, String> = HashMap::new();
                                if let Some(headers) = &msg.headers {
                                    for (k, vals) in headers.iter() {
                                        let key = k.to_string();
                                        let value = vals.iter().map(|hv| hv.to_string()).collect::<Vec<_>>().join(",");
                                        if !value.is_empty() {
                                            h.insert(key, value);
                                        }
                                    }
                                }

                                context::scope_with_headers(&h, async {
                                    if let Err(e) = executor.execute(cmd).await {
                                        error!("[agent] NATS command execution failed: {e}");
                                    }
                                })
                                .await;
                            }
                            Err(e) => {
                                warn!("[agent] failed to decode AiCommandExecute from NATS: {e}");
                            }
                        }
                    }
                }
            }
        })
    }
}
