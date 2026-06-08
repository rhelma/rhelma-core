#![allow(clippy::all, clippy::pedantic, clippy::nursery)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rhelma_core::RequestContext;

fn bench_request_context_from_headers_full(c: &mut Criterion) {
    // Keep these literals stable so the benchmark stays deterministic.
    let headers = [
        (
            "x-rhelma-request-id",
            "123e4567-e89b-12d3-a456-426614174000",
        ),
        (
            "x-rhelma-correlation-id",
            "123e4567-e89b-12d3-a456-426614174001",
        ),
        ("x-rhelma-tenant-id", "acme-corp"),
        ("x-rhelma-residency", "global"),
        (
            "traceparent",
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
        ),
    ];

    c.bench_function("RequestContext::from_headers(full)", |b| {
        b.iter(|| black_box(RequestContext::from_headers(headers).unwrap()))
    });
}

fn bench_request_context_from_headers_minimal(c: &mut Criterion) {
    let headers = [(
        "x-rhelma-request-id",
        "123e4567-e89b-12d3-a456-426614174000",
    )];

    c.bench_function("RequestContext::from_headers(minimal)", |b| {
        b.iter(|| black_box(RequestContext::from_headers(headers).unwrap()))
    });
}

fn bench_request_context_from_headers_empty(c: &mut Criterion) {
    let headers: [(&str, &str); 0] = [];

    c.bench_function("RequestContext::from_headers(empty)", |b| {
        b.iter(|| black_box(RequestContext::from_headers(headers).unwrap()))
    });
}

criterion_group!(
    benches,
    bench_request_context_from_headers_full,
    bench_request_context_from_headers_minimal,
    bench_request_context_from_headers_empty
);
criterion_main!(benches);
