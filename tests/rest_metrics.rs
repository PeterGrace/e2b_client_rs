//! `GET /sandboxes/{id}/metrics` against a mock server.

mod common;

use chrono::{DateTime, Utc};
use common::{SANDBOX_ID, client_for};
use e2b_client::{Error, MetricsParams};
use serde_json::json;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn metric_json() -> serde_json::Value {
    json!({
        "timestamp": "2026-10-27T12:00:05Z",
        "timestampUnix": 1_793_102_405,
        "cpuCount": 2,
        "cpuUsedPct": 12.5,
        "memUsed": 104_857_600,
        "memTotal": 536_870_912,
        "memCache": 1_048_576,
        "diskUsed": 1_073_741_824,
        "diskTotal": 2_147_483_648_u64
    })
}

#[tokio::test]
async fn get_metrics_sends_start_and_end_as_unix_seconds() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/metrics")))
        .and(query_param("start", "1793102400"))
        .and(query_param("end", "1793102460"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([metric_json()])))
        .expect(1)
        .mount(&server)
        .await;
    let start: DateTime<Utc> = "2026-10-27T12:00:00Z".parse().expect("start");
    let end: DateTime<Utc> = "2026-10-27T12:01:00Z".parse().expect("end");

    // Act
    let metrics = client_for(&server)
        .get_sandbox_metrics(SANDBOX_ID, &MetricsParams::new().start(start).end(end))
        .await
        .expect("metrics");

    // Assert
    assert_eq!(metrics.len(), 1);
    let sample = &metrics[0];
    assert_eq!(
        sample.timestamp,
        "2026-10-27T12:00:05Z".parse::<DateTime<Utc>>().expect("ts")
    );
    assert_eq!(sample.cpu_count, 2);
    assert!((sample.cpu_used_pct - 12.5).abs() < f64::EPSILON);
    assert_eq!(sample.mem_used, 104_857_600);
    assert_eq!(sample.mem_total, 536_870_912);
    assert_eq!(sample.mem_cache, 1_048_576);
    assert_eq!(sample.disk_used, 1_073_741_824);
    assert_eq!(sample.disk_total, 2_147_483_648);
}

#[tokio::test]
async fn get_metrics_without_a_range_sends_no_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/metrics")))
        .and(query_param_is_missing("start"))
        .and(query_param_is_missing("end"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;

    let metrics = client_for(&server)
        .get_sandbox_metrics(SANDBOX_ID, &MetricsParams::default())
        .await
        .expect("metrics");

    assert!(metrics.is_empty());
}

#[tokio::test]
async fn get_metrics_of_a_missing_sandbox_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/metrics")))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(json!({"code": 404, "message": "nope"})),
        )
        .mount(&server)
        .await;

    let error = client_for(&server)
        .get_sandbox_metrics(SANDBOX_ID, &MetricsParams::default())
        .await
        .expect_err("not found");

    assert!(
        matches!(error, Error::SandboxNotFound(ref id) if id == SANDBOX_ID),
        "{error:?}"
    );
}
