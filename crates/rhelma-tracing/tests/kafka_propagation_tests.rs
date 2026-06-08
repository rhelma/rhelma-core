#![forbid(unsafe_code)]

#[cfg(feature = "kafka")]
mod kafka {
    use std::collections::HashMap;

    use opentelemetry::propagation::{
        BaggagePropagator, Extractor, Injector, TraceContextPropagator,
    };
    use opentelemetry::trace::TraceContextExt;
    use opentelemetry::Context;
    use rdkafka::message::OwnedHeaders;

    use rhelma_tracing::context::sanitize_baggage_header_value;
    use rhelma_tracing::kafka_propagation::{extract_trace_context, inject_trace_context};

    fn ctx_from_headers_map(headers: &HashMap<String, String>) -> Context {
        let sanitized_baggage: Option<String> = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("baggage"))
            .and_then(|(_, v)| sanitize_baggage_header_value(v));

        struct MapExtractor<'a> {
            headers: &'a HashMap<String, String>,
            sanitized_baggage: Option<String>,
        }

        impl<'a> Extractor for MapExtractor<'a> {
            fn get(&self, key: &str) -> Option<&str> {
                if key.eq_ignore_ascii_case("baggage") {
                    return self.sanitized_baggage.as_deref();
                }
                self.headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(key))
                    .map(|(_, v)| v.as_str())
            }

            fn keys(&self) -> Vec<&str> {
                Vec::new()
            }
        }

        let ex = MapExtractor {
            headers,
            sanitized_baggage,
        };

        let trace_ctx = TraceContextPropagator::new().extract(&ex);
        BaggagePropagator::new().extract_with_context(&trace_ctx, &ex)
    }

    #[test]
    fn kafka_header_inject_and_extract_roundtrip() {
        let mut in_headers = HashMap::new();
        in_headers.insert(
            "traceparent".to_string(),
            "00-0123456789abcdef0123456789abcdef-0123456789abcdef-01".to_string(),
        );
        in_headers.insert(
            "tracestate".to_string(),
            "rojo=00f067aa0ba902b7".to_string(),
        );
        in_headers.insert(
            "baggage".to_string(),
            "rhelma.operation=credit_earn,rhelma.tenant=t1,rhelma.value.amount=42,user.email=alice@example.com".to_string(),
        );

        let cx = ctx_from_headers_map(&in_headers);

        let mut kafka_headers = OwnedHeaders::new();
        inject_trace_context(&mut kafka_headers, &cx);

        let out = extract_trace_context(&kafka_headers);

        let sc = out.span().span_context();
        assert!(sc.is_valid());
        assert_eq!(
            sc.trace_id().to_string(),
            "0123456789abcdef0123456789abcdef"
        );
        assert_eq!(sc.trace_state().header(), "rojo=00f067aa0ba902b7");

        // Ensure baggage is sanitized.
        let mut injected: HashMap<String, String> = HashMap::new();
        struct MapInjector<'a>(&'a mut HashMap<String, String>);
        impl<'a> Injector for MapInjector<'a> {
            fn set(&mut self, key: &str, value: String) {
                self.0.insert(key.to_string(), value);
            }
        }

        BaggagePropagator::new().inject_context(&out, &mut MapInjector(&mut injected));
        let baggage = injected.get("baggage").cloned().unwrap_or_default();
        assert!(baggage.contains("rhelma.operation=credit_earn"));
        assert!(baggage.contains("rhelma.tenant=t1"));
        assert!(baggage.contains("rhelma.value.amount=42"));
        assert!(!baggage.contains("user.email"));
    }
}
