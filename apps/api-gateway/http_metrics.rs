
        // Unified HTTP request metrics collection
        use metrics::increment_counter;
        use axum::http::Request;

        pub async fn http_metrics_middleware(req: Request<Body>, next: Next<Body>) -> Result<Response, Infallible> {
            let start = Instant::now();
            let response = next.run(req).await;
            let duration = start.elapsed();

            // Recording the metrics for the endpoint
            increment_counter!("http_requests_total", "status" => "success", "method" => req.method().as_str());

            Ok(response)
        }
        