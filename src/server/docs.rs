use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::error::Result;
use crate::index::ops;
use crate::server::AppState;

#[derive(Deserialize)]
pub struct ListDocsQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    10
}

pub async fn add_doc(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(doc): Json<JsonValue>,
) -> Result<Json<JsonValue>> {
    let handle = state.manager.open_index(&name).await?;
    let id = ops::add_document(&handle, &doc).await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn bulk_add_docs(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(docs): Json<Vec<JsonValue>>,
) -> Result<Json<JsonValue>> {
    let handle = state.manager.open_index(&name).await?;
    let count = ops::add_documents(&handle, &docs).await?;
    Ok(Json(serde_json::json!({ "count": count })))
}

pub async fn list_docs(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<ListDocsQuery>,
) -> Result<Json<Vec<JsonValue>>> {
    let handle = state.manager.open_index(&name).await?;
    let docs = ops::list_documents(&handle, q.limit, q.offset).await?;
    Ok(Json(docs))
}

pub async fn get_doc(
    State(state): State<AppState>,
    Path((name, field, value)): Path<(String, String, String)>,
) -> Result<Json<JsonValue>> {
    let handle = state.manager.open_index(&name).await?;
    let doc = ops::get_document(&handle, Some(&field), &value).await?;
    Ok(Json(doc))
}

pub async fn delete_doc(
    State(state): State<AppState>,
    Path((name, field, value)): Path<(String, String, String)>,
) -> Result<impl IntoResponse> {
    let handle = state.manager.open_index(&name).await?;
    ops::delete_documents(&handle, &field, &value).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "status": "scheduled" })),
    ))
}
