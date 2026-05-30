use axum::body::Body;
use axum::extract::Json as AxumJson;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::{Json, Router as AxumRouter};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use srvcs_inrange::{api::Deps, health, router, telemetry};
use tower::ServiceExt;

const DEAD_URL: &str = "http://127.0.0.1:1";

/// Spawn a *computing* mock `srvcs-between`: reads `{"value", "lo", "hi"}` and
/// returns `{"result": lo <= value <= hi}` — the real range comparison as a
/// boolean. The inrange orchestrator is genuinely driven by this answer rather
/// than a canned value.
async fn spawn_between() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let value = body.get("value").and_then(Value::as_f64).unwrap_or(0.0);
            let lo = body.get("lo").and_then(Value::as_f64).unwrap_or(0.0);
            let hi = body.get("hi").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": lo <= value && value <= hi }))
        }),
    );
    serve(app).await
}

/// Spawn a mock returning a fixed status + body (used for error-path tests).
async fn spawn_fixed(status: StatusCode, body: Value) -> String {
    let app = AxumRouter::new().route(
        "/",
        post(move || {
            let body = body.clone();
            async move { (status, Json(body)) }
        }),
    );
    serve(app).await
}

async fn serve(app: AxumRouter) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

fn app(between_url: &str) -> axum::Router {
    router(
        telemetry::metrics_handle_for_tests(),
        Deps {
            between_url: between_url.to_string(),
        },
    )
}

async fn inrange(between_url: &str, value: f64, lo: f64, hi: f64) -> (StatusCode, Value) {
    let res = app(between_url)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "value": value, "lo": lo, "hi": hi }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn status_of(uri: &str) -> StatusCode {
    app(DEAD_URL)
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

fn result_bool(body: &Value) -> bool {
    body["result"].as_bool().expect("result is a boolean")
}

// --- Standard endpoints. ---

#[tokio::test]
async fn healthz_ok() {
    assert_eq!(status_of("/healthz").await, StatusCode::OK);
}

#[tokio::test]
async fn readyz_reflects_state() {
    health::set_ready(true);
    assert_eq!(status_of("/readyz").await, StatusCode::OK);
}

#[tokio::test]
async fn metrics_ok() {
    assert_eq!(status_of("/metrics").await, StatusCode::OK);
}

#[tokio::test]
async fn openapi_ok() {
    assert_eq!(status_of("/openapi.json").await, StatusCode::OK);
}

#[tokio::test]
async fn generates_request_id_when_absent() {
    let res = app(DEAD_URL)
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        res.headers().contains_key("x-request-id"),
        "response must carry a generated x-request-id"
    );
}

#[tokio::test]
async fn index_reports_identity() {
    let res = app(DEAD_URL)
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["service"], "srvcs-inrange");
    assert_eq!(body["concern"], "range: is value within [lo, hi]");
    assert_eq!(body["depends_on"], json!(["srvcs-between"]));
}

// --- Correctness cases, against the computing mock. ---

#[tokio::test]
async fn inrange_5_in_0_10_is_true() {
    let between = spawn_between().await;
    let (status, body) = inrange(&between, 5.0, 0.0, 10.0).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["value"], 5.0);
    assert_eq!(body["lo"], 0.0);
    assert_eq!(body["hi"], 10.0);
    assert!(result_bool(&body));
}

#[tokio::test]
async fn inrange_15_in_0_10_is_false() {
    let between = spawn_between().await;
    let (status, body) = inrange(&between, 15.0, 0.0, 10.0).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!result_bool(&body));
}

#[tokio::test]
async fn inrange_includes_lower_bound() {
    let between = spawn_between().await;
    let (status, body) = inrange(&between, 0.0, 0.0, 10.0).await;
    assert_eq!(status, StatusCode::OK);
    assert!(result_bool(&body));
}

#[tokio::test]
async fn inrange_includes_upper_bound() {
    let between = spawn_between().await;
    let (status, body) = inrange(&between, 10.0, 0.0, 10.0).await;
    assert_eq!(status, StatusCode::OK);
    assert!(result_bool(&body));
}

#[tokio::test]
async fn inrange_below_lower_bound_is_false() {
    let between = spawn_between().await;
    let (status, body) = inrange(&between, -1.0, 0.0, 10.0).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!result_bool(&body));
}

// --- Error / degraded paths. ---

#[tokio::test]
async fn degrades_when_between_unreachable() {
    let (status, body) = inrange(DEAD_URL, 5.0, 0.0, 10.0).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-between");
}

#[tokio::test]
async fn forwards_422_from_between() {
    let between = spawn_fixed(
        StatusCode::UNPROCESSABLE_ENTITY,
        json!({ "error": "value is not a number" }),
    )
    .await;
    let (status, _) = inrange(&between, 5.0, 0.0, 10.0).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn degrades_when_between_returns_unexpected_status() {
    let between = spawn_fixed(StatusCode::INTERNAL_SERVER_ERROR, json!({})).await;
    let (status, body) = inrange(&between, 5.0, 0.0, 10.0).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-between");
}
