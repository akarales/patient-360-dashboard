//! Routes: the mini-Copilot surface — patients, 360 view, priority
//! queue, synthesis, review workflow.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::{prioritize, synthesize};
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/patients", get(patients))
        .route("/api/v1/queue", get(queue))
        .route("/api/v1/patients/{patient_id}/360", get(view_360))
        .route("/api/v1/patients/{patient_id}/synthesis", post(synthesis))
        .route("/api/v1/reviews/{review_id}", post(review_action))
        .with_state(state)
}

pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

pub async fn patients(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "patients": state.patients().iter().map(|id| json!({
            "id": id,
            "sources": state.signals_for(id).iter()
                .map(|s| s.source.label()).collect::<std::collections::BTreeSet<_>>()
                .into_iter().collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    }))
}

/// Physician priority queue: pending review items, risk-first.
pub async fn queue(State(state): State<AppState>) -> Json<Value> {
    let reviews = state.all_reviews();
    let ranked: Vec<Value> = prioritize(&reviews)
        .into_iter()
        .map(|item| {
            json!({
                "id": item.id,
                "patient_id": item.patient_id,
                "risk_score": item.risk_score,
                "rationale": item.rationale,
                "state": item.state,
                "created_at": item.created_at,
            })
        })
        .collect();
    Json(json!({
        "queue": ranked,
        "pending": reviews.iter().filter(|i| i.state == crate::domain::ReviewState::Pending).count(),
        "total": reviews.len(),
    }))
}

/// The 360 view: all signals by source + pending items for the patient.
pub async fn view_360(State(state): State<AppState>, Path(patient_id): Path<String>) -> Response {
    let signals = state.signals_for(&patient_id);
    if signals.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("unknown patient: {patient_id}") })),
        )
            .into_response();
    }
    let by_source: serde_json::Map<String, Value> = {
        let mut map = serde_json::Map::new();
        for signal in &signals {
            let entry = map
                .entry(signal.source.label().to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
            if let Value::Array(list) = entry {
                list.push(json!({
                    "code": signal.code,
                    "label": signal.label,
                    "value": signal.value,
                    "unit": signal.unit,
                    "taken_at": signal.taken_at.to_string(),
                }));
            }
        }
        map
    };
    let pending: Vec<Value> = state
        .all_reviews()
        .iter()
        .filter(|item| item.patient_id == patient_id)
        .map(|item| {
            json!({
                "id": item.id,
                "risk_score": item.risk_score,
                "rationale": item.rationale,
                "state": item.state,
            })
        })
        .collect();
    (
        StatusCode::OK,
        Json(json!({
            "patient_id": patient_id,
            "sources": by_source,
            "review_items": pending,
        })),
    )
        .into_response()
}

/// AI narrative for a patient's highest-risk pending item (stub mode).
pub async fn synthesis(State(state): State<AppState>, Path(patient_id): Path<String>) -> Response {
    let reviews: Vec<crate::domain::ReviewItem> = state
        .all_reviews()
        .into_iter()
        .filter(|item| item.patient_id == patient_id)
        .collect();
    let Some(item) = prioritize(&reviews).first().map(|item| (*item).clone()) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "no review items for this patient" })),
        )
            .into_response();
    };
    (
        StatusCode::OK,
        Json(json!({
            "patient_id": patient_id,
            "review_id": item.id,
            "synthesis": synthesize(&patient_id, &item),
            "risk_score": item.risk_score,
            "model": "stub",
            "disclaimer": "Demo synthesis from synthetic data. Not medical advice.",
        })),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
pub struct ReviewAction {
    pub action: String,
}

/// Physician review workflow: acknowledge / escalate a queue item.
pub async fn review_action(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
    Json(action): Json<ReviewAction>,
) -> Response {
    let next = match action.action.as_str() {
        "acknowledge" => crate::domain::ReviewState::Acknowledged,
        "escalate" => crate::domain::ReviewState::Escalated,
        other => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("unknown action: {other}") })),
            )
                .into_response();
        }
    };
    match state.set_review_state(&review_id, next) {
        Some(item) => (
            StatusCode::OK,
            Json(json!({
                "id": item.id,
                "patient_id": item.patient_id,
                "state": item.state,
                "risk_score": item.risk_score,
            })),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("unknown review: {review_id}") })),
        )
            .into_response(),
    }
}
