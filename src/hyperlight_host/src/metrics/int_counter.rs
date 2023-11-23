use prometheus::{
    core::{AtomicU64, GenericCounter},
    register_int_counter_with_registry,
};

use super::{
    get_metric_opts, get_metrics_registry, GetHyperlightMetric, HyperlightMetric,
    HyperlightMetricOps,
};
use crate::{new_error, HyperlightError, Result};

/// A named counter backed by an `AtomicU64`
#[derive(Debug)]
pub struct IntCounter {
    counter: GenericCounter<AtomicU64>,
    /// The name of the counter
    pub name: &'static str,
}

impl IntCounter {
    /// Creates a new counter and registers it with the metric registry
    pub fn new(name: &'static str, help: &str) -> Result<Self> {
        let registry = get_metrics_registry();
        let opts = get_metric_opts(name, help);
        let counter = register_int_counter_with_registry!(opts, registry)?;
        Ok(Self { counter, name })
    }
    /// Increments a counter by 1
    pub fn inc(&self) {
        self.counter.inc();
    }
    /// Increments a counter by a value
    pub fn inc_by(&self, val: u64) {
        self.counter.inc_by(val);
    }
    /// Gets the value of a counter
    pub fn get(&self) -> u64 {
        self.counter.get()
    }
    /// Resets a counter
    pub fn reset(&self) {
        self.counter.reset();
    }
}

impl<S: HyperlightMetricOps> GetHyperlightMetric<IntCounter> for S {
    fn metric(&self) -> Result<&IntCounter> {
        let metric = self.get_metric()?;
        <&HyperlightMetric as TryInto<&IntCounter>>::try_into(metric)
    }
}

impl<'a> TryFrom<&'a HyperlightMetric> for &'a IntCounter {
    type Error = HyperlightError;
    fn try_from(metric: &'a HyperlightMetric) -> Result<Self> {
        match metric {
            HyperlightMetric::IntCounter(counter) => Ok(counter),
            _ => Err(new_error!("metric is not a IntCounter")),
        }
    }
}

impl From<IntCounter> for HyperlightMetric {
    fn from(counter: IntCounter) -> Self {
        HyperlightMetric::IntCounter(counter)
    }
}

/// Increments an IntCounter by 1 or logs an error if the metric is not found
#[macro_export]
macro_rules! int_counter_inc {
    ($metric:expr) => {{
        match $crate::metrics::GetHyperlightMetric::<$crate::metrics::IntCounter>::metric($metric) {
            Ok(val) => val.inc(),
            Err(e) => log::error!("error getting metric: {}", e),
        };
    }};
}

/// Increments an IntCounter by a given value or logs an error if the metric is not found
#[macro_export]
macro_rules! int_counter_inc_by {
    ($metric:expr, $val:expr) => {{
        match $crate::metrics::GetHyperlightMetric::<$crate::metrics::IntCounter>::metric($metric) {
            Ok(val) => val.inc_by($val),
            Err(e) => log::error!("error getting metric: {}", e),
        };
    }};
}

/// Gets the value of an IntCounter or logs an error if the metric is not found
#[macro_export]
macro_rules! int_counter_get {
    ($metric:expr) => {{
        match $crate::metrics::GetHyperlightMetric::<$crate::metrics::IntCounter>::metric($metric) {
            Ok(val) => val.get(),
            Err(e) => {
                log::error!("error getting metric: {}", e);
                0
            }
        }
    }};
}

/// Resets an IntCounter or logs an error if the metric is not found
#[macro_export]
macro_rules! int_counter_reset {
    ($metric:expr) => {{
        match $crate::metrics::GetHyperlightMetric::<$crate::metrics::IntCounter>::metric($metric) {
            Ok(val) => val.reset(),
            Err(e) => log::error!("error getting metric: {}", e),
        };
    }};
}
