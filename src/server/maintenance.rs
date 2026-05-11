use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::error::Result;
use crate::index::ops;
use crate::server::AppState;

pub async fn index_stats(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ops::IndexStats>> {
    let handle = state.manager.open_index(&name).await?;
    let stats = ops::index_stats(&handle).await?;
    Ok(Json(stats))
}

pub async fn rebuild_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse> {
    let handle = state.manager.open_index(&name).await?;
    ops::rebuild_index(&handle).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "status": "rebuilt" })),
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
