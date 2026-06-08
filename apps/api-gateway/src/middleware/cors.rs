use axum::http::{HeaderValue, Method};
use tower_http::cors::{Any, CorsLayer};

use crate::config::CorsConfig;

/// Build CORS layer from GatewayConfig.cors.
/// Fields are: allow_origins, allow_credentials.
pub fn create_cors_layer(_environment: &str, config: &CorsConfig) -> CorsLayer {
    let allow_any = config.allow_origins.iter().any(|o| o == "*");

    // In dev, if allow_origins contains "*", allow Any.
    let mut layer = if allow_any {
        CorsLayer::new().allow_origin(Any)
    } else {
        let origins: Vec<HeaderValue> = config
            .allow_origins
            .iter()
            .filter_map(|o| HeaderValue::from_str(o).ok())
            .collect();
        CorsLayer::new().allow_origin(origins)
    };

    layer = layer.allow_methods([
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
        Method::OPTIONS,
    ]);

    if config.allow_credentials {
        layer = layer.allow_credentials(true);
    }

    layer
}
