use crate::config::TracingConfig;
use tracing_subscriber::Layer;

#[cfg(feature = "otel")]
mod otlp;

/// Initialize OTEL layer (OTLP gRPC).
///
/// This is generic over subscriber type `S` so it can be attached to an
/// already-layered base subscriber (Registry + filters + fmt, etc.).
///
/// When OTEL is disabled by config, this returns a no-op layer.
#[cfg(feature = "otel")]
pub fn init_otel_layer<S>(
    cfg: &TracingConfig,
) -> Result<Box<dyn Layer<S> + Send + Sync>, Box<dyn std::error::Error + Send + Sync>>
where
    S: tracing::Subscriber
        + for<'span> tracing_subscriber::registry::LookupSpan<'span>
        + Send
        + Sync,
{
    if !cfg.otel_enabled {
        return Ok(Box::new(tracing_subscriber::layer::Identity::new()));
    }

    Ok(Box::new(otlp::init_otel_layer::<S>(cfg)?))
}

/// Stub for builds without the `otel` feature.
///
/// Callers are expected to gate OTEL usage behind `cfg(feature = "otel")`.
#[cfg(not(feature = "otel"))]
pub fn init_otel_layer<S>(
    _cfg: &TracingConfig,
) -> Result<Box<dyn Layer<S> + Send + Sync>, Box<dyn std::error::Error + Send + Sync>>
where
    S: tracing::Subscriber,
{
    Ok(Box::new(tracing_subscriber::layer::Identity::new()))
}
