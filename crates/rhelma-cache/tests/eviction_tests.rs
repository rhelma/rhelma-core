// crates/rhelma-cache/tests/eviction_tests.rs
use rhelma_cache::prelude::*;
use std::time::Duration;

#[tokio::test]
async fn test_memory_cache_eviction() {
    // Create a small cache with capacity 2
    let mut cfg = CacheConfig::default();
    cfg.memory.max_capacity = 2;
    let service = CacheService::memory(cfg);

    // Fill the cache
    service
        .set("key1", &"value1", Some(Duration::from_secs(10)))
        .await
        .unwrap();
    service
        .set("key2", &"value2", Some(Duration::from_secs(10)))
        .await
        .unwrap();

    // Both should be in cache
    assert!(service.exists("key1").await.unwrap());
    assert!(service.exists("key2").await.unwrap());

    // Add third item - should evict the least recently used (key1)
    service
        .set("key3", &"value3", Some(Duration::from_secs(10)))
        .await
        .unwrap();

    // key1 should be evicted, key2 and key3 should remain
    assert!(!service.exists("key1").await.unwrap());
    assert!(service.exists("key2").await.unwrap());
    assert!(service.exists("key3").await.unwrap());
}

#[tokio::test]
async fn test_memory_cache_ttl_eviction() {
    let mut cfg = CacheConfig::default();
    cfg.memory.max_capacity = 10;
    let service = CacheService::memory(cfg);

    // Set item with short TTL
    service
        .set("temp", &"temporary", Some(Duration::from_millis(10)))
        .await
        .unwrap();

    // Should exist initially
    assert!(service.exists("temp").await.unwrap());

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Should be evicted due to TTL
    assert!(!service.exists("temp").await.unwrap());
}
