
# Rhelma Operations

## Deployment Steps
1. Set up PostgreSQL and Redis.
2. Configure environment variables in `.env` file.
3. Build the system: `cargo build --release`
4. Run the system: `cargo run`

## Monitoring
- **Prometheus**: Collect metrics from the system.
- **Grafana**: Visualize the metrics and system health.

## Troubleshooting
1. If the system fails to start, check the logs in `rhelma-logger`.
2. If Redis or NATS are not available, the system may not function properly. Ensure these services are running.
