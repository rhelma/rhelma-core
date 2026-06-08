//! main.rs — Rhelma Observability Agent CLI Bootstrap (v5.2, Kafka + Reflex-ready)
//!
//! Patch v5.2-F1:
//! - Wildcard/regex topic subscription is forbidden.
//! - Use explicit topic allow-lists (CSV) via OBS_AGENT_*_TOPICS.

use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use rhelma_event::EventBus;
use rhelma_event_kafka_agent::{KafkaBusConfig, KafkaConfig, KafkaEventBus};
use rhelma_observability_core::ObservabilityCore;

use rhelma_observability_agent::{
    agent::config::ObservabilityAgentConfig,
    error::AgentError,
    io::{KafkaCommandSource, KafkaDecisionSource, KafkaSignalSource, NatsCommandSource},
    reflex::anomaly::NaiveAnomalyDetector,
    runtime::{DecisionSource, ExternalCommandSource},
    AgentRuntime, ObservabilityAgent,
};

#[tokio::main]
async fn main() -> Result<(), AgentError> {
    eprintln!("Starting Rhelma Observability Agent (v5.2, Kafka + Reflex)…");

    // 1) Load central env + agent config
    let central = rhelma_config::CentralEnv::from_env_strict()
        .map_err(|e| AgentError::internal(format!("central env: {e}")))?;
    let cfg = ObservabilityAgentConfig::from_central(&central)?;

    // 1.1) Init global observability (logger/tracing/metrics) using the shared core.
    // This prevents double-initialization and keeps behavior consistent across services.
    let unified =
        rhelma_config::UnifiedObservabilityConfig::from_central_env(&central, &cfg.service_name);
    rhelma_config::validation::validate_all(&unified, &central)
        .map_err(|e| AgentError::internal(format!("config validation: {e}")))?;
    let _core = ObservabilityCore::init_from_unified(unified)
        .await
        .map_err(|e| AgentError::internal(format!("observability core init failed: {e}")))?;

    info!("🚀 Observability core initialized");

    // 2) Build EventBus (Kafka) — outbound events
    let bus = build_event_bus(&cfg)?;

    // 3) Build runtime
    let mut runtime = AgentRuntime::new(cfg.clone(), bus.clone())?;

    // 3.1) Attach Reflex Signal Source (Kafka)
    if let Some(signal_src) = build_signal_source(&cfg, bus.clone(), runtime.agent()) {
        runtime.attach_signal_source(signal_src);
        info!("🧠 Reflex signal loop enabled (KafkaSignalSource)");
    } else {
        warn!("🧠 Reflex signal loop disabled (no signal source / init failed)");
    }

    // 4) Command source (Kafka/NATS/None)
    let command_source = build_command_source::<KafkaEventBus>(&cfg).await;

    // 5) Decision source (Kafka/None)
    let decision_source = build_decision_source::<KafkaEventBus>(&cfg);

    // 6) Start full runtime
    info!("✅ Agent runtime starting (heartbeat + optional loops) …");
    runtime.run(command_source, decision_source).await?;

    Ok(())
}

//
// Shared helpers
//

fn kafka_bootstrap(cfg: &ObservabilityAgentConfig) -> String {
    env::var("KAFKA_BOOTSTRAP_SERVERS")
        .ok()
        .or_else(|| cfg.kafka_bootstrap.clone())
        .unwrap_or_else(|| "localhost:9092".into())
}

fn topic_prefix_env() -> Option<String> {
    env::var("OBS_AGENT_TOPIC_PREFIX").ok()
}

fn parse_topics_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

fn topics_from_env(list_env: &str, single_env: &str, default: Vec<String>) -> Vec<String> {
    if let Ok(csv) = env::var(list_env) {
        let v = parse_topics_csv(&csv);
        if !v.is_empty() {
            return v;
        }
    }

    if let Ok(one) = env::var(single_env) {
        let t = one.trim();
        if !t.is_empty() {
            return vec![t.to_string()];
        }
    }

    default
}

fn has_wildcard_or_regex(topics: &[String]) -> Option<String> {
    topics
        .iter()
        .find(|t| t.contains('*') || t.starts_with('^'))
        .cloned()
}

//
// EventBus factory (Kafka) — outbound publish
//

fn build_event_bus(cfg: &ObservabilityAgentConfig) -> Result<Arc<KafkaEventBus>, AgentError> {
    let bootstrap = kafka_bootstrap(cfg);

    let client_id =
        env::var("OBS_AGENT_CLIENT_ID").unwrap_or_else(|_| "rhelma-observability-agent".into());

    let topic_prefix = topic_prefix_env();

    let bus_cfg = KafkaBusConfig {
        bootstrap_servers: bootstrap,
        client_id,
        topic_prefix,
        ..KafkaBusConfig::default()
    };

    let bus = KafkaEventBus::new(bus_cfg)?;
    Ok(Arc::new(bus))
}

//
// Reflex Signal Source (Kafka) — inbound obs.signal -> detector -> actions
//

fn build_signal_source<B>(
    cfg: &ObservabilityAgentConfig,
    bus: Arc<B>,
    agent: Arc<ObservabilityAgent<B>>,
) -> Option<KafkaSignalSource<B>>
where
    B: EventBus + Send + Sync + 'static,
{
    let mode = env::var("OBS_AGENT_SIGNAL_SOURCE").unwrap_or_else(|_| "kafka".into());
    if mode.to_lowercase() == "none" {
        return None;
    }

    // topics WITHOUT prefix؛ چون KafkaSubscriber خودش topic_prefix را اضافه می‌کند
    let topics = topics_from_env(
        "OBS_AGENT_SIGNAL_TOPICS",
        "OBS_AGENT_SIGNAL_TOPIC",
        vec!["obs.signal".into()],
    );

    if let Some(bad) = has_wildcard_or_regex(&topics) {
        error!("[agent] signal topic contains wildcard/regex: {bad} (use explicit list)");
        return None;
    }

    let group = env::var("OBS_AGENT_SIGNAL_GROUP").unwrap_or_else(|_| {
        cfg.kafka_group_id
            .clone()
            .unwrap_or_else(|| "rhelma-agent-signals".into())
    });

    let topic_prefix = topic_prefix_env().unwrap_or_default();

    let kcfg = KafkaConfig {
        brokers: kafka_bootstrap(cfg),
        group_id: group,
        topic_prefix,
        ..Default::default()
    };

    let detector = Arc::new(Mutex::new(NaiveAnomalyDetector::new(
        cfg.service_name.clone(),
        cfg.service_version.clone(),
        cfg.environment.clone(),
        cfg.region.clone(),
        cfg.residency_mode.clone(),
        bus.clone(),
        cfg.anomaly_window_size,
    )));

    match KafkaSignalSource::new_many(kcfg, topics, agent, detector) {
        Ok(src) => Some(src),
        Err(e) => {
            error!("[agent] Failed to init KafkaSignalSource: {e}");
            None
        }
    }
}

//
// Command source factory (Kafka/NATS/None)
//

async fn build_command_source<B>(
    cfg: &ObservabilityAgentConfig,
) -> Option<Arc<dyn ExternalCommandSource<B>>>
where
    B: EventBus + Send + Sync + 'static,
{
    let default_mode = if cfg.command_enabled { "kafka" } else { "none" };
    let mode = env::var("OBS_AGENT_COMMAND_SOURCE").unwrap_or_else(|_| default_mode.into());

    match mode.to_lowercase().as_str() {
        "kafka" => {
            let bootstrap = kafka_bootstrap(cfg);

            let group = env::var("OBS_AGENT_COMMAND_GROUP").unwrap_or_else(|_| {
                cfg.kafka_group_id
                    .clone()
                    .unwrap_or_else(|| "rhelma-agent-commands".into())
            });

            let default_topic = cfg
                .kafka_command_topic
                .clone()
                .unwrap_or_else(|| "ai.command.execute".into());

            let topics = topics_from_env(
                "OBS_AGENT_COMMAND_TOPICS",
                "OBS_AGENT_COMMAND_TOPIC",
                vec![default_topic],
            );

            if let Some(bad) = has_wildcard_or_regex(&topics) {
                error!("[agent] command topic contains wildcard/regex: {bad} (use explicit list)");
                return None;
            }

            KafkaCommandSource::new_many(&bootstrap, &group, topics)
                .map(|src| Arc::new(src) as _)
                .map_err(|e| {
                    error!("[agent] Failed to init KafkaCommandSource: {e}");
                    e
                })
                .ok()
        }

        "nats" => {
            let url = env::var("NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".into());
            let subject = env::var("OBS_AGENT_COMMAND_SUBJECT")
                .unwrap_or_else(|_| "ai.command.execute".into());

            NatsCommandSource::<B>::connect(&url, &subject)
                .await
                .map(|src| Arc::new(src) as _)
                .map_err(|e| {
                    error!("[agent] Failed to init NatsCommandSource: {e}");
                    e
                })
                .ok()
        }

        _ => None,
    }
}

//
// Decision Source (Kafka/None)
//

fn build_decision_source<B>(cfg: &ObservabilityAgentConfig) -> Option<Arc<dyn DecisionSource<B>>>
where
    B: EventBus + Send + Sync + 'static,
{
    if !cfg.decision_enabled {
        return None;
    }

    let bootstrap = kafka_bootstrap(cfg);

    let group = env::var("OBS_AGENT_DECISION_GROUP").unwrap_or_else(|_| {
        cfg.kafka_group_id
            .clone()
            .unwrap_or_else(|| "rhelma-agent-decisions".into())
    });

    let default_topic = cfg
        .kafka_decision_topic
        .clone()
        .unwrap_or_else(|| "ai.incident.decision".into());

    let topics = topics_from_env(
        "OBS_AGENT_DECISION_TOPICS",
        "OBS_AGENT_DECISION_TOPIC",
        vec![default_topic],
    );

    if let Some(bad) = has_wildcard_or_regex(&topics) {
        error!("[agent] decision topic contains wildcard/regex: {bad} (use explicit list)");
        return None;
    }

    KafkaDecisionSource::new_many(&bootstrap, &group, topics)
        .map(|src| Arc::new(src) as _)
        .map_err(|e| {
            error!("[agent] Failed to init KafkaDecisionSource: {e}");
            e
        })
        .ok()
}
