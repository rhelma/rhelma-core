// crates/rhelma-cache/src/macros.rs
use sha2::{Sha256, Digest};

/// Macro for caching the result of an async function
#[macro_export]
macro_rules! cached {
    ($cache:expr, $key:expr, $ttl:expr, $block:expr) => {{
        use $crate::CacheService;
        
        if let Some(cached) = $cache.get($key).await? {
            return Ok(cached);
        }
        
        let result = $block.await?;
        $cache.set($key, &result, $ttl).await?;
        Ok(result)
    }};
}

/// Macro for caching with automatic key generation using cryptographic hash
#[macro_export]
macro_rules! cached_fn {
    ($cache:expr, $fn_name:expr, $($arg:expr),*, $ttl:expr, $block:expr) => {{
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update($fn_name.as_bytes());
        $( hasher.update($arg.to_string().as_bytes()); )*
        let key = format!("{}:{}", $fn_name, hex::encode(hasher.finalize()));
        
        $crate::cached!($cache, &key, $ttl, $block)
    }};
}
