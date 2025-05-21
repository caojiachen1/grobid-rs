# First-class Cloud Observability for Grobid-RS

## Motivation

Production and enterprise users require robust observability to monitor, alert, and scale Grobid-RS deployments. Integrating Prometheus and OpenTelemetry (OTEL) metrics enables seamless integration with modern cloud and DevOps stacks, making Grobid-RS a first-class citizen in production environments.

## Technical Background

- **Prometheus**: Widely used open-source monitoring system. Scrapes `/metrics` endpoints for time-series data.
- **OpenTelemetry (OTEL)**: Standard for distributed tracing and metrics. Supported by all major cloud providers.
- **metrics-exporter-prometheus**: Rust crate for exposing Prometheus metrics.
- **tracing**: Rust crate for structured, async-aware logging and spans.

## Implementation Plan

### 1. Metrics Exporter Integration
- Add `metrics` and `metrics-exporter-prometheus` crates as dependencies.
- Register a Prometheus exporter at server startup.
- Expose `/metrics` endpoint in Axum server.

Example:
```rust
use metrics_exporter_prometheus::PrometheusBuilder;

let builder = PrometheusBuilder::new();
let recorder = builder.install_recorder().unwrap();
// In Axum route:
async fn metrics() -> impl IntoResponse {
    recorder.render()
}
```

### 2. Instrumentation
- Use `metrics!` macros to record counters, histograms, and gauges:
  - PDF parse time
  - Model inference time
  - Request/response counts
  - Error counts
- Use `tracing` for span-based logging and distributed tracing.

### 3. Security Considerations
- Restrict `/metrics` endpoint to trusted networks (optional)
- Document how to secure metrics in production

### 4. Testing & Documentation
- Add integration tests for metrics endpoint
- Document available metrics and how to scrape with Prometheus
- Provide example Grafana dashboards

## Example Metrics
```
grobid_rs_pdf_parse_seconds_bucket{le="0.1"} 42
grobid_rs_pdf_parse_seconds_bucket{le="0.5"} 100
grobid_rs_pdf_parse_seconds_count 150
grobid_rs_pdf_parse_seconds_sum 23.5
grobid_rs_inference_seconds_bucket{le="0.1"} 30
```

## Quick Wins
- `/metrics` endpoint with basic counters
- Prometheus scrape config in README

## References
- [metrics crate](https://docs.rs/metrics/)
- [metrics-exporter-prometheus](https://docs.rs/metrics-exporter-prometheus/)
- [tracing crate](https://docs.rs/tracing/)
- [OpenTelemetry](https://opentelemetry.io/) 