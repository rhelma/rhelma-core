#![forbid(unsafe_code)]

#[cfg(feature = "otel")]
mod otel {
    use std::collections::HashMap;

    use opentelemetry::propagation::{BaggagePropagator, Injector};
    use opentelemetry::trace::TraceContextExt;
    use rhelma_event_kafka::otel_context_from_headers_map;

    #[test]
    fn extracts_parent_span_context_from_traceparent() {
        let mut headers = HashMap::new();
        headers.insert(
            "traceparent".to_string(),
            "00-0123456789abcdef0123456789abcdef-0123456789abcdef-01".to_string(),
        );
        headers.insert(
            "tracestate".to_string(),
            "rojo=00f067aa0ba902b7".to_string(),
        );

        headers.insert(
            "baggage".to_string(),
            // Include a non-allowlisted key to ensure consumer-side sanitization is applied.
            "rhelma.operation=credit_earn,rhelma.tenant=t1,rhelma.value.amount=42,user.email=alice@example.com".to_string(),
        );

        let ctx = otel_context_from_headers_map(&headers);
        let sc = ctx.span().span_context();

        assert!(sc.is_valid());
        assert_eq!(
            sc.trace_id().to_string(),
            "0123456789abcdef0123456789abcdef"
        );
        assert_eq!(sc.span_id().to_string(), "0123456789abcdef");
        assert!(sc.trace_flags().is_sampled());
        assert_eq!(sc.trace_state().header(), "rojo=00f067aa0ba902b7");

        // Ensure baggage is present in the extracted context by re-injecting it.
        let mut out: HashMap<String, String> = HashMap::new();
        struct MapInjector<'a>(&'a mut HashMap<String, String>);
        impl<'a> Injector for MapInjector<'a> {
            fn set(&mut self, key: &str, value: String) {
                self.0.insert(key.to_string(), value);
            }
        }
        BaggagePropagator::new().inject_context(&ctx, &mut MapInjector(&mut out));
        let baggage = out.get("baggage").cloned().unwrap_or_default();
        assert!(baggage.contains("rhelma.operation=credit_earn"));
        assert!(baggage.contains("rhelma.tenant=t1"));
        assert!(baggage.contains("rhelma.value.amount=42"));
        assert!(!baggage.contains("user.email"));
    }
}
