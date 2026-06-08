
        // Enhanced metrics with OpenTelemetry
        use axum::http::Request;
        use tracing::info;

        pub async fn http_metrics_middleware(req: Request<Body>, next: Next<Body>) -> Result<Response, Infallible> {
            let start = Instant::now();
            let response = next.run(req).await;
            let duration = start.elapsed();

            info!("Request to {} took {:?}", req.uri(), duration);

            Ok(response)
        }
        