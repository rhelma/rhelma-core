// crates/rhelma-cache/src/macros.rs
/// Macro for caching the result of an async function
#[macro_export]
macro_rules! cached {
    ($cache:expr, $key:expr, $ttl:expr, $block:expr) => {{
        use $crate::metrics::{self, CacheBackendKind};
        use $crate::tracing_ext::cache_span;
        use $crate::CacheService;

        let span = cache_span("cached", "unknown", None);
        let _guard = span.enter();

        // cache lookup
        if let Some(cached) = $cache.get($key).await? {
            metrics::record_hit(CacheBackendKind::Other("unknown"), "get", "cached_macro");
            return Ok(cached);
        }
        metrics::record_miss(CacheBackendKind::Other("unknown"), "get", "cached_macro");

        let result = $block.await?;
        $cache.set($key, &result, $ttl).await?;
        Ok(result)
    }};
}

/// Macro for caching the result of a function with automatic key generation
#[macro_export]
macro_rules! cached_fn {
    ($cache:expr, $fn_name:expr, $ttl:expr, $( $arg:expr ),* , $block:expr) => {{
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        $fn_name.hash(&mut hasher);
        $( $arg.hash(&mut hasher); )*
        let key = format!("{}:{}", $fn_name, hasher.finish());

        $crate::cached!($cache, &key, $ttl, $block)
    }};
}

/// Expression-oriented caching macro that does not `return` from the outer function.
#[macro_export]
macro_rules! cached_value {
    ($cache:expr, $key:expr, $ttl:expr, $block:expr) => {{
        use $crate::metrics::{self, CacheBackendKind};
        use $crate::tracing_ext::cache_span;

        let span = cache_span("cached_value", "unknown", None);
        let _guard = span.enter();

        if let Some(cached) = $cache.get($key).await? {
            metrics::record_hit(
                CacheBackendKind::Other("unknown"),
                "get",
                "cached_value_macro",
            );
            Ok(cached)
        } else {
            metrics::record_miss(
                CacheBackendKind::Other("unknown"),
                "get",
                "cached_value_macro",
            );
            let result = $block.await?;
            $cache.set($key, &result, $ttl).await?;
            Ok(result)
        }
    }};
}
