# Observability

Hyperlight provides the following observability features:

* [Metrics](#metrics) metrics are provided using Prometheus.
* [Logs](#logs) are provided using the Rust [log crate](https://docs.rs/log/0.4.6/log/), and can be consumed by any Rust logger implementation, including LogTracer which can be used to emit log records as tracing events.
* [Tracing](#tracing) is provided using the Rust [tracing crate](https://docs.rs/tracing/0.1.25/tracing/), and can be consumed by any Rust tracing implementation. In addition the [log feature](https://docs.rs/tracing/latest/tracing/#crate-feature-flags) is enabled which means that should a hyperlight host application not want to consume tracing events, you can still consume them as logs.

## Metrics

Hyperlight provides metrics using Prometheus. The metrics are registered using either the [default_registry](https://docs.rs/prometheus/latest/prometheus/fn.default_registry.html) or a registry instance provided by the host application.

To provide a registry to Hyperlight, use the `set_metrics_registry` function and pass a reference to a registry with `static` lifetime:

```rust
use hyperlight_host::metrics::set_metrics_registry;
use prometheus::Registry;
use lazy_static::lazy_static;

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
}

set_metrics_registry(&REGISTRY);
```

The following metrics are provided and are enabled by default:

* `hyperlight_current_number_of_multi_use_sandboxes` - a gauge that tracks the number of multi-use sandboxes that are currently active.
* `hyperlight_current_number_of_single_use_sandboxes` - a gauge that tracks the number of single-use sandboxes that are currently active.
* `hyperlight_guest_error_count` - a vector of counters that tracks the number of guest errors by code and message.
* `hyperlight_number_of_cancelled_guest_execution` - a counter that tracks the number of guest executions that have been cancelled because the execution time execeeded the time allowed.

The following metrics are provided but are disabled by default and require the feature `function_call_metrics` to be enabled:

* `hyperlight_guest_function_call_duration_microseconds` - a vector of histograms that tracks the execution time of guest functions in microseconds by function name. The histogram also tracks the number of calls to each function.
* `hyperlight_host_function_calls_duration_microseconds` - a vector of histograms that tracks the execution time of host functions in microseconds by function name. The histogram also tracks the number of calls to each function.

The rationale for disabling the function call metrics by default is that:

* A Hyperlight host may wish to provide its own metrics for function calls.
* Enabling a trace subscriber will cause the function call metrics to be emitted as trace events, which may be sufficient for some use cases.

There is an example of how to gather metrics in the [examples/metrics](../src/hyperlight_host/examples/metrics) directory.

The metrics capabilties provided by Hyperlight can also be used by a library or host that is using Hyperlight to provide additional metrics, see the [hypervisor metrics module](../src/hyperlight_host/src/hypervisor/metrics.rs) for an example of how to define metrics.

## Logs

Hyperlight provides logs using the Rust [log crate](https://docs.rs/log/0.4.6/log/), and can be consumed by any Rust logger implementation, including LogTracer which can be used to emit log records as tracing events(see below for more details). To consume logs, the host application must provide a logger implementation either by using the `set_logger` function directly or using a logger implementation that is compatible with the log crate.

For an example that uses the `env_logger` crate, see the [examples/logging](../src/hyperlight_host/examples/logging) directory. By default the `env_logger` crate will only log messages at the `error` level or higher. To see all log messages, set the `RUST_LOG` environment variable to `debug`.

Hyperlight also provides tracing capabilties (see below for more details), if no trace subscriber is registered, trace records will be emitted as log records, using the `log` feature of the [tracing crate](https://docs.rs/tracing/latest/tracing/#crate-feature-flags).

## Tracing

Tracing spans are created for any call to a public API and the parent span will be set to the current span in the host if one exists, the level of the span is set to `info`. The span will be closed when the call returns. Any Result that contains an error variant will be logged as an error event. In addition to the public APIs, all internal functions are instrumented with trace spans at the `trace` level, therefore in order to see full trace information, the trace level should be enabled.

Hyperlight provides tracing using the Rust [tracing crate](https://docs.rs/tracing/0.1.25/tracing/), and can be consumed by any Rust trace subscriber implementation(see[here](https://docs.rs/tracing/latest/tracing/index.html#related-crates) for some examples). In addition to consuming trace output the log records may also be consumed by a tracing subscriber, using the `tracing-log` crate.

There is an example that shows how to consume both tracing events and log records as tracing events in the [examples/tracing](../src/hyperlight_host/examples/tracing) directory.
