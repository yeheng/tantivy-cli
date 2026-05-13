use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use std::sync::atomic::Ordering;

use crate::error::Result;
use crate::index::handle::IndexStatus;
use crate::index::ops;
use crate::server::AppState;

pub async fn index_stats(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let handle = state.manager.open_index(&name).await?;

    let status = handle.status.load(Ordering::Acquire);
    if status == IndexStatus::REBUILDING_U8 {
        return Ok(Json(serde_json::json!({ "status": "rebuilding" })));
    }

    let stats = ops::index_stats(&handle).await?;
    Ok(Json(serde_json::to_value(stats).unwrap()))
}

pub async fn rebuild_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse> {
    let handle = state.manager.open_index(&name).await?;
    ops::trigger_rebuild(handle)?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "status": "rebuilding_started" })),
    ))
}

pub async fn compress_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse> {
    let handle = state.manager.open_index(&name).await?;
    ops::compress_index(&handle).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "status": "compressed" })),
    ))
}
