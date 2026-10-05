//! Integration tests over the seeded state.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use patient_360_dashboard::routes;
use patient_360_dashboard::state::AppState;

fn app() -> axum::Router {
    routes::router(AppState::seeded())
}

async fn get(path: &str) -> (StatusCode, Value) {
    let response = app()
        .oneshot(Request::get(path).body(Body::empty()).expect("builds"))
        .await
        .expect("request");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    (status, serde_json::from_slice(&bytes).expect("json"))
}

async fn post_json(path: &str, body: Value) -> (StatusCode, Value) {
    let response = app()
        .oneshot(
            Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("builds"),
        )
        .await
        .expect("request");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    (status, serde_json::from_slice(&bytes).expect("json"))
}

#[tokio::test]
async fn health_ok() {
    let (status, body) = get("/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn queue_prioritizes_highest_risk_first() {
    let (status, body) = get("/api/v1/queue").await;
    assert_eq!(status, StatusCode::OK);
    let queue = body["queue"].as_array().expect("queue");
    assert!(!queue.is_empty(), "seed data produces pending items");
    // linus (healthy control) has no items.
    assert!(queue.iter().all(|item| item["patient_id"] != "linus"));
    let scores: Vec<f64> = queue
        .iter()
        .map(|item| item["risk_score"].as_f64().expect("score"))
        .collect();
    let mut sorted = scores.clone();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
    assert_eq!(scores, sorted, "queue is risk-sorted");
}

#[tokio::test]
async fn view_360_groups_by_source() {
    let (status, body) = get("/api/v1/patients/grace/360").await;
    assert_eq!(status, StatusCode::OK);
    let sources = body["sources"].as_object().expect("sources");
    assert!(sources.contains_key("vitals"));
    assert!(sources.contains_key("wearables"));
    assert_eq!(body["review_items"].as_array().expect("items").len(), 2);
}

#[tokio::test]
async fn view_360_unknown_patient_404() {
    let (status, _) = get("/api/v1/patients/nobody/360").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn synthesis_is_disclaimed_and_risk_tied() {
    let (status, body) = post_json("/api/v1/patients/ada/synthesis", serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let text = body["synthesis"].as_str().expect("text");
    assert!(text.contains("not medical advice"));
    assert!(body["risk_score"].as_f64().expect("score") > 0.0);
}

#[tokio::test]
async fn review_workflow_acknowledge() {
    let (_, queue) = get("/api/v1/queue").await;
    let top = queue["queue"][0]["id"]
        .as_str()
        .expect("item id")
        .to_string();

    // The workflow is stateful on a shared store; use one app instance.
    let app = app();
    let response = app
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/reviews/{top}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"action":"acknowledge"}"#))
                .expect("builds"),
        )
        .await
        .expect("request");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let body: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(body["state"], "acknowledged");

    let response = app
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/reviews/{top}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"action":"escalate"}"#))
                .expect("builds"),
        )
        .await
        .expect("request");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let body: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(body["state"], "escalated");
}

#[tokio::test]
async fn review_unknown_action_rejected() {
    let (status, _) = post_json(
        "/api/v1/reviews/rev-ada-labs",
        serde_json::json!({ "action": "zap" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
