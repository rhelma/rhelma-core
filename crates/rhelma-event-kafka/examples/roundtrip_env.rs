//! KafkaConfig env roundtrip example.
//!
//! Historical note: earlier versions exposed helper functions like
//! `to_env_prefix`/`from_env_prefix`. In v5.2, those helpers were removed in
//! favor of unified configuration wiring.

use rhelma_event_kafka::KafkaConfig;

fn main() {
    let cfg = KafkaConfig::default();

    if let Err(e) = cfg.validate_for_producer() {
        eprintln!("producer config validation failed: {e}");
        std::process::exit(1);
    }

    if let Err(e) = cfg.validate_for_consumer() {
        eprintln!("consumer config validation failed: {e}");
        std::process::exit(1);
    }

    println!("KafkaConfig validated: {cfg:?}");
}
