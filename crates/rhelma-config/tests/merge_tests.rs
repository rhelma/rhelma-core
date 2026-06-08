use rhelma_config::merge::{deep_merge, flattened_to_nested};
use serde_json::{json, Map, Value};

#[test]
fn deep_merge_overrides_scalars() {
    let a = json!({"a": 1, "nested": {"x": 1}});
    let b = json!({"b": 2, "nested": {"y": 2}});

    let merged = deep_merge(a, b);
    assert_eq!(merged["a"], 1);
    assert_eq!(merged["b"], 2);
    assert_eq!(merged["nested"]["x"], 1);
    assert_eq!(merged["nested"]["y"], 2);
}

#[test]
fn flattened_to_nested_builds_objects() {
    let mut flat = Map::new();
    flat.insert("logger.level".into(), Value::from("info"));
    flat.insert("tracing.sampling_rate".into(), Value::from(0.5));

    let nested = flattened_to_nested(flat);

    assert_eq!(nested["logger"]["level"], "info");
    assert_eq!(nested["tracing"]["sampling_rate"], 0.5);
}
