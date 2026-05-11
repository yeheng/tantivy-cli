use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

impl From<SortOrder> for tantivy::Order {
    fn from(o: SortOrder) -> Self {
        match o {
            SortOrder::Asc => tantivy::Order::Asc,
            SortOrder::Desc => tantivy::Order::Desc,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct EsSearchRequest {
    #[serde(alias = "offset", default)]
    pub from: usize,
    #[serde(alias = "limit", default = "default_size")]
    pub size: usize,
    pub query: Option<EsQuery>,
    #[serde(default)]
    pub sort: Vec<HashMap<String, SortOrder>>,
    #[serde(default)]
    pub aggs: Option<tantivy::aggregation::agg_req::Aggregations>,
    #[serde(default)]
    pub highlight_fields: Vec<String>,
    #[serde(default = "default_snippet_chars")]
    pub snippet_max_chars: usize,
    #[serde(default)]
    pub _source: Option<Vec<String>>,
}

fn default_size() -> usize {
    10
}
fn default_snippet_chars() -> usize {
    150
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EsQuery {
    Bool {
        #[serde(default)]
        must: Vec<EsQuery>,
        #[serde(default)]
        filter: Vec<EsQuery>,
        #[serde(default)]
        should: Vec<EsQuery>,
        #[serde(default)]
        must_not: Vec<EsQuery>,
    },
    Match(HashMap<String, String>),
    Term(HashMap<String, JsonValue>),
    Range(HashMap<String, RangeParams>),
    QueryString {
        query: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct RangeParams {
    #[serde(default)]
    pub gte: Option<JsonValue>,
    #[serde(default)]
    pub gt: Option<JsonValue>,
    #[serde(default)]
    pub lte: Option<JsonValue>,
    #[serde(default)]
    pub lt: Option<JsonValue>,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub doc: JsonValue,
    pub score: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippets: Option<JsonValue>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub total: usize,
    pub hits: Vec<SearchHit>,
    pub query: String,
    pub limit: usize,
    pub offset: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregations: Option<JsonValue>,
}
