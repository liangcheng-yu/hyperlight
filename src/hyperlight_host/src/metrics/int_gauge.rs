use prometheus::{
    core::{AtomicI64, GenericGauge},
    register_int_gauge_with_registry,
};

use super::{
    get_metric_opts, get_metrics_registry, GetHyperlightMetric, HyperlightMetric,
    HyperlightMetricOps,
};
use crate::{new_error, HyperlightError, Result};

/// A gauge backed by an `AtomicI64`
#[derive(Debug)]
pub struct IntGauge {
    gauge: GenericGauge<AtomicI64>,
    /// The name of the gauge
    pub name: &'static str,
}

impl IntGauge {
    /// Creates a new gauge and registers it with the metric registry
    pub fn new(name: &'static str, help: &str) -> Result<Self> {
        let registry = get_metrics_registry();
        let opts = get_metric_opts(name, help);
        let gauge = register_int_gauge_with_registry!(opts, registry)?;
        Ok(Self { gauge, name })
    }
    /// Increments a gauge by 1
    pub fn inc(&self) {
        self.gauge.inc();
    }
    /// Decrements a gauge by 1
    pub fn dec(&self) {
        self.gauge.dec();
    }
    /// Gets the value of a gauge
    pub fn set(&self, val: i64) {
        self.gauge.set(val);
    }
    /// Resets a gauge
    pub fn get(&self) -> i64 {
        self.gauge.get()
    }
    /// Adds a value to a gauge
    pub fn add(&self, val: i64) {
        self.gauge.add(val);
    }
    /// Subtracts a value from a gauge
    pub fn sub(&self, val: i64) {
        self.gauge.sub(val)
    }
}

impl<S: HyperlightMetricOps> GetHyperlightMetric<IntGauge> for S {
    fn metric(&self) -> Result<&IntGauge> {
        let metric = self.get_metric()?;
        <&HyperlightMetric as TryInto<&IntGauge>>::try_into(metric)
    }
}

impl<'a> TryFrom<&'a HyperlightMetric> for &'a IntGauge {
    type Error = HyperlightError;
    fn try_from(metric: &'a HyperlightMetric) -> Result<Self> {
        match metric {
            HyperlightMetric::IntGauge(gauge) => Ok(gauge),
            _ => Err(new_error!("metric is not a IntGauge")),
        }
    }
}

impl From<IntGauge> for HyperlightMetric {
    fn from(gauge: IntGauge) -> Self {
        HyperlightMetric::IntGauge(gauge)
    }
}

/// Increments an IntGauge by 1 or logs an error if the metric is not found
#[macro_export]
macro_rules! int_gauge_inc {
    ($metric:expr) => {{
        match $crate::metrics::GetHyperlightMetric::<$crate::metrics::IntGauge>::metric($metric) {
            Ok(val) => val.inc(),
            Err(e) => log::error!("error getting metric: {}", e),
        };
    }};
}

/// Decrements an IntGauge by 1 or logs an error if the metric is not found
#[macro_export]
macro_rules! int_gauge_dec {
    ($metric:expr) => {{
        match $crate::metrics::GetHyperlightMetric::<$crate::metrics::IntGauge>::metric($metric) {
            Ok(val) => val.dec(),
            Err(e) => log::error!("error getting metric: {}", e),
        };
    }};
}

/// Sets an IntGauge to value or logs an error if the metric is not found
#[macro_export]
macro_rules! int_gauge_set {
    ($metric:expr, $val:expr) => {{
        match $crate::metrics::GetHyperlightMetric::<$crate::metrics::IntGauge>::metric($metric) {
            Ok(val) => val.set($val),
            Err(e) => log::error!("error getting metric: {}", e),
        };
    }};
}

/// Gets the value of an IntGauge logs an error
/// and returns 0 if the metric is not found
#[macro_export]
macro_rules! int_gauge_get {
    ($metric:expr) => {{
        match $crate::metrics::GetHyperlightMetric::<$crate::metrics::IntGauge>::metric($metric) {
            Ok(val) => val.get(),
            Err(e) => {
                log::error!("error getting metric: {}", e);
                0
            }
        }
    }};
}

/// Adds a value to an IntGauge or logs an error if the metric is not found
#[macro_export]
macro_rules! int_gauge_add {
    ($metric:expr, $val:expr) => {{
        match $crate::metrics::GetHyperlightMetric::<$crate::metrics::IntGauge>::metric($metric) {
            Ok(val) => val.add($val),
            Err(e) => log::error!("error getting metric: {}", e),
        };
    }};
}

/// Subtracts a value from an IntGauge or logs an error if the metric is not found
#[macro_export]
macro_rules! int_gauge_sub {
    ($metric:expr, $val:expr) => {{
        match $crate::metrics::GetHyperlightMetric::<$crate::metrics::IntGauge>::metric($metric) {
            Ok(val) => val.sub($val),
            Err(e) => log::error!("error getting metric: {}", e),
        };
    }};
}
