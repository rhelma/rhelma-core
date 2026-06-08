#![forbid(unsafe_code)]

use crate::state::AppState;
use crate::ws::ws_handler;
use axum::{routing::get, Router};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(ws_handler))
}




