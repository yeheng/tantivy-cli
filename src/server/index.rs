use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::error::Result;
use crate::index::ops;
use crate::index::schema::SchemaDef;
use crate::server::AppState;

#[derive(Deserialize)]
pub struct CreateIndexReq {
    pub schema: SchemaDef,
}

#[derive(Serialize)]
pub struct IndexInfo {
    pub name: String,
    pub num_docs: u64,
    pub schema: JsonValue,
}

pub async fn create_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<CreateIndexReq>,
) -> Result<impl IntoResponse> {
    state.manager.create_index(&name, &req.schema).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "index": name })),
    ))
}

pub async fn list_indexes(State(state): State<AppState>) -> Result<Json<Vec<String>>> {
    state.manager.load_all_indexes()?;
    Ok(Json(state.manager.list_indexes()))
}

pub async fn delete_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse> {
    state.manager.delete_index(&name).await?;
    Ok((StatusCode::NO_CONTENT, ()))
}

pub async fn get_index_info(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<IndexInfo>> {
    let handle = state.manager.open_index(&name).await?;
    let stats = ops::index_stats(&handle).await?;
    Ok(Json(IndexInfo {
        name,
        num_docs: stats.num_docs,
        schema: stats.schema,
    }))
}
