//! Example: integrating rhelma-tracing context with an HTTP-style middleware.
//!
//! This example does not spin up a real server; instead it shows how you can
//! plug `rhelma_tracing::context` into your own middleware stack using a
//! `http::Request` and `http::Response` pair.

use http::{Request, Response};
use rhelma_tracing::{context, prelude::*};

fn handle_with_tracing(req: Request<()>) -> Response<&'static str> {
    // --- Incoming side: extract trace/correlation headers ---
    let mut incoming_headers = std::collections::HashMap::new();
    for (name, value) in req.headers().iter() {
        if let Ok(val) = value.to_str() {
            incoming_headers.insert(name.to_string(), val.to_string());
        }
    }
    context::extract_traceparent(&incoming_headers);
    context::extract_current_context(&incoming_headers);

    // Optionally generate a correlation id for this logical request.
    if context::current_correlation_id().is_none() {
        context::set_correlation_id(format!("req-{}", uuid::Uuid::now_v7()));
    }

    // --- Business logic span ---
    let span = instrument_span!("http_request", "path" => req.uri().path().to_string());
    let _guard = span.enter();
    info!("handling HTTP request");

    // --- Outgoing side: inject updated context into response headers ---
    let mut response = Response::builder().status(200).body("ok").unwrap();

    let mut outgoing_headers = std::collections::HashMap::new();
    context::inject_traceparent(&mut outgoing_headers);
    context::inject_current_context(&mut outgoing_headers);

    for (k, v) in outgoing_headers {
        response.headers_mut().insert(
            http::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
            http::HeaderValue::from_str(&v).unwrap(),
        );
    }

    response
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cfg = TracingConfig::default().with_service_name("example-http-middleware".to_string());

    let _tracing = RhelmaTracing::init("example-http-middleware", cfg).await?;

    let req = Request::builder()
        .uri("https://example.com/api/demo")
        .body(())
        .unwrap();

    let resp = handle_with_tracing(req);
    println!("status = {}", resp.status());
    Ok(())
}
