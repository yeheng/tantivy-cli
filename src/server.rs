use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::error::Result;
use crate::index::manager::IndexManager;
use crate::index::ops;
use crate::index::schema::SchemaDef;
use crate::search::{search_index, SearchRequest, SearchResponse};

#[derive(Clone)]
pub struct AppState {
    pub manager: IndexManager,
}

pub async fn serve(manager: IndexManager, bind: &str) -> Result<()> {
    // Start background task to periodically clean up expired documents.
    let cleanup_manager = manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let handles = cleanup_manager.iter_handles();
            for handle in handles {
                if handle.expired_at_field.is_some() {
                    if let Err(e) = ops::cleanup_expired(&handle).await {
                        tracing::error!(index = %handle.name, error = %e, "failed to cleanup expired documents");
                    }
                }
            }
        }
    });

    let state = AppState { manager };

    let app = Router::new()
        .route("/indexes", get(list_indexes))
        .route("/indexes/{name}", post(create_index).delete(delete_index).get(get_index_info))
        .route("/indexes/{name}/docs", post(add_doc).get(list_docs))
        .route("/indexes/{name}/docs/{field}/{value}", get(get_doc).delete(delete_doc))
        .route("/indexes/{name}/search", get(search).post(search_post))
        .route("/indexes/{name}/stats", get(index_stats))
        .route("/indexes/{name}/rebuild", post(rebuild_index))
        .route("/indexes/{name}/compress", post(compress_index))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// ---- Index handlers ----

#[derive(Deserialize)]
struct CreateIndexReq {
    schema: SchemaDef,
}

async fn create_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<CreateIndexReq>,
) -> Result<impl IntoResponse> {
    state.manager.create_index(&name, &req.schema).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "index": name }))))
}

async fn list_indexes(State(state): State<AppState>) -> Result<Json<Vec<String>>> {
    state.manager.load_all_indexes()?;
    Ok(Json(state.manager.list_indexes()))
}

async fn delete_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse> {
    state.manager.delete_index(&name).await?;
    Ok((StatusCode::NO_CONTENT, ()))
}

#[derive(Serialize)]
struct IndexInfo {
    name: String,
    num_docs: u64,
    schema: JsonValue,
}

async fn get_index_info(
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

// ---- Document handlers ----

async fn add_doc(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(doc): Json<JsonValue>,
) -> Result<Json<JsonValue>> {
    let handle = state.manager.open_index(&name).await?;
    let id = ops::add_document(&handle, &doc).await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

#[derive(Deserialize)]
struct ListDocsQuery {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

fn default_limit() -> usize {
    10
}

async fn list_docs(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<ListDocsQuery>,
) -> Result<Json<Vec<JsonValue>>> {
    let handle = state.manager.open_index(&name).await?;
    let docs = ops::list_documents(&handle, q.limit, q.offset).await?;
    Ok(Json(docs))
}

async fn get_doc(
    State(state): State<AppState>,
    Path((name, field, value)): Path<(String, String, String)>,
) -> Result<Json<JsonValue>> {
    let handle = state.manager.open_index(&name).await?;
    let doc = ops::get_document(&handle, Some(&field), &value).await?;
    Ok(Json(doc))
}

async fn delete_doc(
    State(state): State<AppState>,
    Path((name, field, value)): Path<(String, String, String)>,
) -> Result<impl IntoResponse> {
    let handle = state.manager.open_index(&name).await?;
    let deleted = ops::delete_documents(&handle, &field, &value).await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "deleted": deleted }))))
}

// ---- Search handlers ----

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    highlight: Vec<String>,
    #[serde(default = "default_snippet")]
    snippet_max_chars: usize,
}

fn default_snippet() -> usize {
    150
}

async fn search(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<SearchResponse>> {
    let handle = state.manager.open_index(&name).await?;
    let req = SearchRequest {
        query: q.q,
        limit: q.limit,
        offset: q.offset,
        highlight_fields: q.highlight,
        snippet_max_chars: q.snippet_max_chars,
    };
    let resp = search_index(&handle, &req).await?;
    Ok(Json(resp))
}

async fn search_post(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>> {
    let handle = state.manager.open_index(&name).await?;
    let resp = search_index(&handle, &req).await?;
    Ok(Json(resp))
}

// ---- Maintenance handlers ----

async fn index_stats(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ops::IndexStats>> {
    let handle = state.manager.open_index(&name).await?;
    let stats = ops::index_stats(&handle).await?;
    Ok(Json(stats))
}

async fn rebuild_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse> {
    let handle = state.manager.open_index(&name).await?;
    ops::rebuild_index(&handle).await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "rebuilt" }))))
}

async fn compress_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse> {
    let handle = state.manager.open_index(&name).await?;
    ops::compress_index(&handle).await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "compressed" }))))
}
