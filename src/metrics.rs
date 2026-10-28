//! Sandbox resource metrics (`GET /sandboxes/{id}/metrics`).

use chrono::{DateTime, Utc};
use serde::Deserialize;

/// One sample of a sandbox's CPU, memory and disk use
/// (`SandboxMetric`, `e2b/api/client/models/sandbox_metric.py:28-36`).
///
/// This is a plain data record, so its fields are public. It is
/// `#[non_exhaustive]` so fields can be added without a breaking change.
///
/// # Examples
///
/// ```no_run
/// use e2b_client::{MetricsParams, Sandbox};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// for sample in sandbox.get_metrics(&MetricsParams::new()).await? {
///     println!(
///         "{} cpu {:.1}% mem {}/{}",
///         sample.timestamp, sample.cpu_used_pct, sample.mem_used, sample.mem_total
///     );
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SandboxMetrics {
    /// When the sample was taken.
    pub timestamp: DateTime<Utc>,
    /// Number of vCPUs.
    pub cpu_count: u32,
    /// CPU use, in percent.
    pub cpu_used_pct: f64,
    /// Memory in use, in bytes.
    pub mem_used: u64,
    /// Total memory, in bytes.
    pub mem_total: u64,
    /// Page cache, in bytes.
    pub mem_cache: u64,
    /// Disk in use, in bytes.
    pub disk_used: u64,
    /// Total disk, in bytes.
    pub disk_total: u64,
}

/// The time range for [`crate::Client::get_sandbox_metrics`]. With neither
/// bound set, the API returns samples from the sandbox's start to now.
///
/// # Examples
///
/// ```
/// use chrono::{DateTime, Duration, Utc};
/// use e2b_client::MetricsParams;
///
/// // The crate builds chrono without its `clock` feature, so this example
/// // parses a fixed time rather than calling `Utc::now()`.
/// let end: DateTime<Utc> = "2026-10-27T12:05:00Z".parse().expect("a valid time");
/// let params = MetricsParams::new().start(end - Duration::minutes(5)).end(end);
/// # let _ = params;
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MetricsParams {
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
}

impl MetricsParams {
    /// No range: the sandbox's whole life so far.
    ///
    /// # Returns
    ///
    /// Empty parameters.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Only samples at or after `start`. Sent in whole Unix seconds.
    ///
    /// # Arguments
    ///
    /// * `start` - the earliest sample time.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn start(mut self, start: DateTime<Utc>) -> Self {
        self.start = Some(start);
        self
    }

    /// Only samples at or before `end`. Sent in whole Unix seconds.
    ///
    /// # Arguments
    ///
    /// * `end` - the latest sample time.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn end(mut self, end: DateTime<Utc>) -> Self {
        self.end = Some(end);
        self
    }

    /// The query pairs (`get_sandboxes_sandbox_id_metrics.py:21-23`).
    pub(crate) fn query(&self) -> Vec<(&'static str, i64)> {
        // `filter_map` drops the unset bounds, like the SDK's `UNSET`.
        [("start", self.start), ("end", self.end)]
            .into_iter()
            .filter_map(|(name, time)| time.map(|time| (name, time.timestamp())))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_without_a_range_is_empty() {
        assert!(MetricsParams::new().query().is_empty());
    }

    #[test]
    fn query_sends_each_set_bound_as_unix_seconds() {
        // Arrange: a sub-second part, which the query drops.
        let start: DateTime<Utc> = "2026-10-27T12:00:00.750Z".parse().expect("start");
        let end: DateTime<Utc> = "2026-10-27T12:01:00Z".parse().expect("end");

        // Act
        let only_end = MetricsParams::new().end(end).query();
        let both = MetricsParams::new().start(start).end(end).query();

        // Assert
        assert_eq!(only_end, vec![("end", 1_793_102_460)]);
        assert_eq!(both, vec![("start", 1_793_102_400), ("end", 1_793_102_460)]);
    }
}
