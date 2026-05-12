use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::error::Result;
use crate::search::{EsQuery, EsSearchRequest, SearchResponse, search_index};
use crate::server::AppState;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub highlight: Vec<String>,
    #[serde(default = "default_snippet")]
    pub snippet_max_chars: usize,
}

fn default_limit() -> usize {
    10
}

fn default_snippet() -> usize {
    150
}

pub async fn search(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<SearchResponse>> {
    let handle = state.manager.open_index(&name).await?;
    let req = EsSearchRequest {
        from: q.offset,
        size: q.limit,
        query: Some(EsQuery::QueryString { query: q.q }),
        sort: Vec::new(),
        aggs: None,
        highlight_fields: q.highlight,
        snippet_max_chars: q.snippet_max_chars,
        _source: None,
    };
    let resp = search_index(handle, &req).await?;
    Ok(Json(resp))
}

pub async fn search_post(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<EsSearchRequest>,
) -> Result<Json<SearchResponse>> {
    let handle = state.manager.open_index(&name).await?;
    let resp = search_index(handle, &req).await?;
    Ok(Json(resp))
}
