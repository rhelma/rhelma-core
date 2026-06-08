#![forbid(unsafe_code)]

use axum::{
    http::header,
    response::{Html, IntoResponse},
};

#[cfg(feature = "openapi")]
use crate::openapi::ApiDoc;

/// OpenAPI document endpoint.
///
/// - Default build: serves an empty placeholder document.
/// - `--features openapi`: generates the document from code (utoipa).
pub async fn openapi_json() -> impl IntoResponse {
    let body: String = {
        #[cfg(feature = "openapi")]
        {
            serde_json::to_string_pretty(&ApiDoc::openapi()).unwrap_or_else(|_| "{}".to_string())
        }

        #[cfg(not(feature = "openapi"))]
        {
            "{}".to_string()
        }
    };

    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        body,
    )
}

/// Minimal landing page for API docs.
pub async fn docs_landing() -> impl IntoResponse {
    let swagger_section = if cfg!(feature = "openapi") {
        r#"<p>
    Swagger UI: <a href=\"/swagger-ui\">/swagger-ui</a>
  </p>"#
    } else {
        r#"<p>
    Embedded Swagger UI is disabled in the default build.
    Build <code>node-registry</code> with <code>--features openapi</code> to enable it.
  </p>"#
    };

    Html(format!(
        r#"<!doctype html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\" />
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />
  <title>Rhelma Node Registry API Docs</title>
  <style>
    body {{ font-family: system-ui, -apple-system, Segoe UI, Roboto, Arial, sans-serif; margin: 40px; }}
    code {{ background: #f5f5f5; padding: 2px 4px; border-radius: 4px; }}
  </style>
</head>
<body>
  <h1>Rhelma Node Registry</h1>
  <p>
    OpenAPI JSON: <a href=\"/api-docs/openapi.json\">/api-docs/openapi.json</a>
  </p>
  {swagger_section}
  <p>
    Tip: you can point external Swagger UI / Redoc at the OpenAPI JSON endpoint above.
  </p>
</body>
</html>"#
    ))
}
