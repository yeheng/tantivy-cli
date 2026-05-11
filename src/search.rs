use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tantivy::{
    collector::{Count, MultiCollector, TopDocs},
    query::QueryParser,
    snippet::SnippetGenerator,
    DocAddress, TantivyDocument,
};

use crate::error::{AppError, Result};
use crate::index::manager::IndexHandle;
use crate::index::ops::doc_to_json;

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub highlight_fields: Vec<String>,
    #[serde(default = "default_snippet_chars")]
    pub snippet_max_chars: usize,
}

fn default_limit() -> usize {
    10
}
fn default_snippet_chars() -> usize {
    150
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub doc: JsonValue,
    pub score: f32,
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
}

pub async fn search_index(handle: &IndexHandle, req: &SearchRequest) -> Result<SearchResponse> {
    let searcher = handle.reader.searcher();

    let text_fields: Vec<_> = handle
        .schema
        .fields()
        .filter(|(_, entry)| matches!(entry.field_type(), tantivy::schema::FieldType::Str(_)))
        .map(|(field, _)| field)
        .collect();

    if text_fields.is_empty() {
        return Err(AppError::Schema("no text fields to search".to_string()));
    }

    let query_parser = QueryParser::for_index(&handle.index, text_fields.clone());
    let query = query_parser
        .parse_query(&req.query)
        .map_err(|e| AppError::Query(e.to_string()))?;

    let top_collector = TopDocs::with_limit(req.limit + req.offset).order_by_score();
    let mut collectors = MultiCollector::new();
    let top_docs_handle = collectors.add_collector(top_collector);
    let count_handle = collectors.add_collector(Count);
    let mut multi_fruit = searcher.search(&query, &collectors)?;
    let total = count_handle.extract(&mut multi_fruit);
    let top_docs: Vec<(f32, DocAddress)> = top_docs_handle.extract(&mut multi_fruit);

    let mut hits = Vec::with_capacity(top_docs.len().saturating_sub(req.offset));

    let mut snippet_gens: Vec<(String, SnippetGenerator)> = Vec::new();
    if !req.highlight_fields.is_empty() {
        for field_name in &req.highlight_fields {
            if let Ok(field) = handle.schema.get_field(field_name) {
                if matches!(
                    handle.schema.get_field_entry(field).field_type(),
                    tantivy::schema::FieldType::Str(_)
                ) {
                    let mut generator =
                        SnippetGenerator::create(&searcher, &*query, field)?;
                    generator.set_max_num_chars(req.snippet_max_chars);
                    snippet_gens.push((field_name.clone(), generator));
                }
            }
        }
    }

    for (idx, (score, doc_address)) in top_docs.into_iter().enumerate() {
        if idx < req.offset {
            continue;
        }
        let doc: TantivyDocument = searcher.doc(doc_address)?;
        let doc_json = doc_to_json(&handle.schema, &doc);

        let mut snippets = serde_json::Map::new();
        for (field_name, generator) in &snippet_gens {
            let snippet = generator.snippet_from_doc(&doc);
            let html = snippet.to_html();
            snippets.insert(field_name.clone(), JsonValue::String(html));
        }

        hits.push(SearchHit {
            doc: doc_json,
            score,
            snippets: if snippets.is_empty() {
                None
            } else {
                Some(JsonValue::Object(snippets))
            },
        });
    }

    Ok(SearchResponse {
        total,
        hits,
        query: req.query.clone(),
        limit: req.limit,
        offset: req.offset,
    })
}
