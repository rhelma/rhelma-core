mod common;
use common::FakeEventBus;

use std::sync::Arc;

use chrono::Utc;
use rhelma_event::contracts::ai::AiCommandExecute;
use rhelma_observability_agent::agent::config::ResidencyMode;
use rhelma_observability_agent::commands::executor::AiCommandResultV2;
use rhelma_observability_agent::commands::CommandExecutor;

#[tokio::test]
async fn command_rejected_when_not_allowed() {
    let bus = Arc::new(FakeEventBus::default());

    let executor = CommandExecutor::new_with_version(
        bus.clone(),
        ResidencyMode::Global,
        "svc".into(),
        "1.0.0".into(),
        "eu".into(),
    );

    let cmd = AiCommandExecute {
        command_id: "cmd-1".into(),
        incident_id: None,
        service: "svc".into(),
        region: "eu".into(),
        action: "rm_rf".into(), // not allowed
        parameters: serde_json::json!({ "path": "/" }),
        requested_at: Utc::now(),
    };

    executor.execute(cmd).await.unwrap();

    let published = bus.published.lock().unwrap();
    let last = published.last().unwrap();

    let payload: AiCommandResultV2 = serde_json::from_value(last.payload.clone()).unwrap();
    assert!(!payload.success);
}

#[tokio::test]
async fn command_allowed_publishes_success() {
    let bus = Arc::new(FakeEventBus::default());

    let executor = CommandExecutor::new_with_version(
        bus.clone(),
        ResidencyMode::Global,
        "svc".into(),
        "1.0.0".into(),
        "eu".into(),
    );

    let cmd = AiCommandExecute {
        command_id: "cmd-2".into(),
        incident_id: None,
        service: "svc".into(),
        region: "eu".into(),
        action: "set_log_level".into(), // allowed
        parameters: serde_json::json!({ "level": "info" }),
        requested_at: Utc::now(),
    };

    executor.execute(cmd).await.unwrap();

    let published = bus.published.lock().unwrap();
    let last = published.last().unwrap();
    let payload: AiCommandResultV2 = serde_json::from_value(last.payload.clone()).unwrap();

    assert!(payload.success);
}
