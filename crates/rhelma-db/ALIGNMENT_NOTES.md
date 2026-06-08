# rhelma-db v5.2 alignment patch (pool + tracing)

## What changed
- **pool.rs**: hardened residency enforcement logic + added `record_pool_gauges()` helper.
- **tracing_ext.rs**: db spans now include Rhelma v5.2 / OTEL-ish context fields (`request.id`, `correlation.id`, `tenant.id`, `user.id`, `region`) while keeping legacy fields.

## How to apply
Copy these files into:
- `crates/rhelma-db/src/pool.rs`
- `crates/rhelma-db/src/tracing_ext.rs`

Then run:
- `cargo test -p rhelma-db`
