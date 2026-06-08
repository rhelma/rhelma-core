use crate::state::AppState;
use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use tracing::instrument;

/// Admin and health endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/info", get(info))
}

#[derive(Serialize)]
struct HealthResponse {
    service: String,
    region: String,
    overall: String,
}

#[instrument(skip(state))]
async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        service: state.config.service_name.clone(),
        region: state.config.region.clone(),
        overall: "healthy".into(),
    })
}

#[derive(Serialize)]
struct InfoResponse {
    service: String,
    environment: String,
    region: String,
    model: String,
}

#[instrument(skip(state))]
async fn info(State(state): State<AppState>) -> Json<InfoResponse> {
    Json(InfoResponse {
        service: state.config.service_name.clone(),
        environment: state.config.environment.clone(),
        region: state.config.region.clone(),
        model: state.config.embedding_model.clone(),
    })
}
