use serde_json::Value as JsonValue;
use tantivy::{DocAddress, TantivyDocument};
use tantivy::snippet::SnippetGenerator;

use crate::error::Result;
use crate::index::manager::IndexHandle;
use crate::index::ops::doc_to_json;
use crate::search::model::{EsSearchRequest, SearchHit};

/// Trait to abstract over `TopDocs` collector results.
pub trait TopDocsResult {
    fn doc_address(&self) -> DocAddress;
    fn to_search_hit(&self, doc_json: JsonValue, snippets: Option<JsonValue>) -> SearchHit;
}

impl TopDocsResult for (f32, DocAddress) {
    fn doc_address(&self) -> DocAddress {
        self.1
    }
    fn to_search_hit(&self, doc_json: JsonValue, snippets: Option<JsonValue>) -> SearchHit {
        SearchHit {
            doc: doc_json,
            score: Some(self.0),
            snippets,
        }
    }
}

impl<T: Clone + Send + Sync + std::fmt::Debug + 'static> TopDocsResult for (Option<T>, DocAddress) {
    fn doc_address(&self) -> DocAddress {
        self.1
    }
    fn to_search_hit(&self, doc_json: JsonValue, snippets: Option<JsonValue>) -> SearchHit {
        SearchHit {
            doc: doc_json,
            score: None,
            snippets,
        }
    }
}

pub fn build_snippet_gens(
    handle: &IndexHandle,
    searcher: &tantivy::Searcher,
    query: &dyn tantivy::query::Query,
    req: &EsSearchRequest,
) -> Result<Vec<(String, SnippetGenerator)>> {
    let mut gens = Vec::new();
    if !req.highlight_fields.is_empty() {
        for field_name in &req.highlight_fields {
            if let Ok(field) = handle.schema.get_field(field_name) {
                if matches!(
                    handle.schema.get_field_entry(field).field_type(),
                    tantivy::schema::FieldType::Str(_)
                ) {
                    let mut generator = SnippetGenerator::create(searcher, query, field)?;
                    generator.set_max_num_chars(req.snippet_max_chars);
                    gens.push((field_name.clone(), generator));
                }
            }
        }
    }
    Ok(gens)
}

pub fn process_top_docs<R: TopDocsResult>(
    handle: &IndexHandle,
    searcher: &tantivy::Searcher,
    req: &EsSearchRequest,
    snippet_gens: &[(String, SnippetGenerator)],
    top_docs: Vec<R>,
) -> Result<Vec<SearchHit>> {
    let mut hits = Vec::with_capacity(top_docs.len().saturating_sub(req.from));
    for (idx, r) in top_docs.into_iter().enumerate() {
        if idx < req.from {
            continue;
        }
        let doc = searcher.doc::<TantivyDocument>(r.doc_address())?;
        let mut doc_json = doc_to_json(&handle.schema, &doc);

        // _source filtering
        if let Some(ref sources) = req._source {
            if let JsonValue::Object(ref mut map) = doc_json {
                map.retain(|k, _| sources.contains(k));
            }
        }

        let mut snippets = serde_json::Map::new();
        for (field_name, generator) in snippet_gens {
            let snippet = generator.snippet_from_doc(&doc);
            snippets.insert(field_name.clone(), JsonValue::String(snippet.to_html()));
        }

        hits.push(r.to_search_hit(
            doc_json,
            if snippets.is_empty() {
                None
            } else {
                Some(JsonValue::Object(snippets))
            },
        ));
    }
    Ok(hits)
}
