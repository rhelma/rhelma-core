#![forbid(unsafe_code)]

use metrics::{counter, gauge, histogram};

pub fn incr_connections_opened() {
    counter!("rt_ws_connections_opened_total").increment(1);
}

pub fn incr_connections_closed() {
    counter!("rt_ws_connections_closed_total").increment(1);
}

pub fn set_active_connections(count: i64) {
    gauge!("rt_ws_active_connections").set(count as f64);
}

pub fn incr_messages_in() {
    counter!("rt_ws_messages_in_total").increment(1);
}

pub fn incr_messages_out() {
    counter!("rt_ws_messages_out_total").increment(1);
}

pub fn incr_messages_rejected() {
    counter!("rt_ws_messages_rejected_total").increment(1);
}

pub fn incr_rate_limit_hit() {
    counter!("rt_ws_rate_limit_hit_total").increment(1);
}

pub fn incr_close(reason: &str, code: Option<u16>) {
    let reason_s = reason.to_string();
    let code_s = code
        .map(|c| c.to_string())
        .unwrap_or_else(|| "none".to_string());
    counter!(
        "rt_ws_connections_closed_by_reason_total",
        "reason" => reason_s,
        "code" => code_s
    )
    .increment(1);
}

pub fn record_connection_duration(seconds: f64) {
    histogram!("rt_ws_connection_duration_seconds").record(seconds);
}
