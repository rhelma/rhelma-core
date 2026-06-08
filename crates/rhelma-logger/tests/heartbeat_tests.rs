use rhelma_logger::{LogBuilder, LogLevel};

#[test]
fn test_heartbeat_event() {
    let event = LogBuilder::new(LogLevel::Info, "hb").heartbeat().build();

    assert_eq!(event.operation_kind.as_deref(), Some("heartbeat"));
    assert_eq!(event.operation_name.as_deref(), Some("system.heartbeat"));
}
