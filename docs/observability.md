# Observability

Hyperlight provides the following observability features:

* [Metrics](#metrics) metrics are provided using Prometheus.
* [Logs](#logs) are provided using the Rust [log crate](https://docs.rs/log/0.4.6/log/), and can be consumed by any Rust logger implementation, including LogTracer which can be used to emit log records as tracing events.
* [Tracing](#tracing) is provided using the Rust [tracing crate](https://docs.rs/tracing/0.1.25/tracing/), and can be consumed by any Rust tracing implementation. In addition the [log feature](https://docs.rs/tracing/latest/tracing/#crate-feature-flags) is enabled which means that should a hyperlight host application not want to consume tracing events, you can still consume them as logs.

## Metrics

Hyperlight provides metrics using Prometheus. The metrics are registered using either the [default_registry](https://docs.rs/prometheus/latest/prometheus/fn.default_registry.html) or a registry instance provided by the host application.

To provide a registry to Hyperlight, use the `set_metrics_registry`function and pass a reference to a registry with `static` lifetime:

```rust
use hyperlight_host::metrics::set_metrics_registry;
use prometheus::Registry;
use lazy_static::lazy_static;

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
}

set_metrics_registry(&REGISTRY);
```

## Logs

## Tracing
