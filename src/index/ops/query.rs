use serde_json::Value as JsonValue;
use tantivy::{
    DocAddress, TantivyDocument,
    query::{TermQuery},
    schema::IndexRecordOption,
};

use crate::error::{AppError, Result};
use crate::index::doc::{doc_to_json, str_to_term};
use crate::index::manager::IndexHandle;

/// Get document by field value.
pub async fn get_document(
    handle: &IndexHandle,
    field_name: &str,
    term_value: &str,
) -> Result<JsonValue> {
    let field_name = field_name.to_string();
    let term_value = term_value.to_string();
    let schema = handle.schema.clone();
    let reader = handle.reader.clone();

    tokio::task::spawn_blocking(move || {
        let searcher = reader.searcher();

        let field = schema
            .get_field(&field_name)
            .map_err(|_| AppError::FieldNotFound(field_name.clone()))?;
        let field_entry = schema.get_field_entry(field);
        let term = str_to_term(field, field_entry.field_type(), &term_value)?;
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        let top_docs: Vec<(f32, DocAddress)> = searcher.search(
            &query,
            &tantivy::collector::TopDocs::with_limit(1).order_by_score(),
        )?;
        let doc = if let Some((_, doc_address)) = top_docs.into_iter().next() {
            searcher.doc::<TantivyDocument>(doc_address)?
        } else {
            return Err(AppError::DocNotFound(term_value));
        };

        Ok(doc_to_json(&schema, &doc))
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))?
}

/// Maximum number of documents that can be retrieved in a single list request.
const MAX_RESULT_WINDOW: usize = 10_000;

/// List all documents (paginated).
/// Note: deep offset is still O(offset) — for true cursor-based pagination,
/// consider using search with a range query on a fast field.
pub async fn list_documents(
    handle: &IndexHandle,
    limit: usize,
    offset: usize,
) -> Result<Vec<JsonValue>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    if offset + limit > MAX_RESULT_WINDOW {
        return Err(AppError::BadRequest(format!(
            "offset + limit cannot exceed {MAX_RESULT_WINDOW}"
        )));
    }

    let schema = handle.schema.clone();
    let reader = handle.reader.clone();

    tokio::task::spawn_blocking(move || {
        let searcher = reader.searcher();
        let total = searcher.num_docs() as usize;
        if offset >= total {
            return Ok(Vec::new());
        }

        let top_docs: Vec<(f32, DocAddress)> = searcher.search(
            &tantivy::query::AllQuery,
            &tantivy::collector::TopDocs::with_limit(offset + limit).order_by_score(),
        )?;

        if offset >= top_docs.len() {
            return Ok(Vec::new());
        }

        let mut docs = Vec::with_capacity(top_docs.len() - offset);
        for (_, doc_address) in &top_docs[offset..] {
            let doc = searcher.doc::<TantivyDocument>(*doc_address)?;
            docs.push(doc_to_json(&schema, &doc));
        }
        Ok(docs)
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))?
}
