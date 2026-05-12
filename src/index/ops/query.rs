use serde_json::Value as JsonValue;
use tantivy::{
    DocAddress, TantivyDocument,
    query::{TermQuery},
    schema::{IndexRecordOption, Term},
};

use crate::error::{AppError, Result};
use crate::index::doc::doc_to_json;
use crate::index::manager::IndexHandle;

/// Get document by field value.
pub async fn get_document(
    handle: &IndexHandle,
    field_name: Option<&str>,
    term_value: &str,
) -> Result<JsonValue> {
    let field_name = field_name.map(|s| s.to_string());
    let term_value = term_value.to_string();
    let schema = handle.schema.clone();
    let reader = handle.reader.clone();

    tokio::task::spawn_blocking(move || {
        let searcher = reader.searcher();

        let doc = if let Some(field_name) = field_name {
            let field = schema
                .get_field(&field_name)
                .map_err(|_| AppError::FieldNotFound(field_name.clone()))?;
            let field_entry = schema.get_field_entry(field);
            let term = match field_entry.field_type() {
                tantivy::schema::FieldType::Str(_) => Term::from_field_text(field, &term_value),
                tantivy::schema::FieldType::U64(_) => Term::from_field_u64(field, term_value.parse()?),
                tantivy::schema::FieldType::I64(_) => Term::from_field_i64(field, term_value.parse()?),
                tantivy::schema::FieldType::F64(_) => Term::from_field_f64(field, term_value.parse()?),
                tantivy::schema::FieldType::Bool(_) => Term::from_field_bool(field, term_value.parse()?),
                tantivy::schema::FieldType::Date(_) => {
                    let dt = term_value
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .map_err(|e| AppError::Schema(format!("invalid date: {e}")))?;
                    Term::from_field_date_for_search(
                        field,
                        tantivy::DateTime::from_timestamp_micros(dt.timestamp_micros()),
                    )
                }
                _ => {
                    return Err(AppError::Schema(
                        "unsupported lookup field type".to_string(),
                    ));
                }
            };
            let query = TermQuery::new(term, IndexRecordOption::Basic);
            let top_docs: Vec<(f32, DocAddress)> = searcher.search(
                &query,
                &tantivy::collector::TopDocs::with_limit(1).order_by_score(),
            )?;
            if let Some((_, doc_address)) = top_docs.into_iter().next() {
                searcher.doc::<TantivyDocument>(doc_address)?
            } else {
                return Err(AppError::DocNotFound(term_value));
            }
        } else {
            return Err(AppError::BadRequest(
                "field_name required for get".to_string(),
            ));
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

        let doc_addresses: Vec<DocAddress> =
            top_docs.into_iter().skip(offset).map(|(_, addr)| addr).collect();

        let mut docs = Vec::with_capacity(doc_addresses.len());
        for doc_address in doc_addresses {
            let doc = searcher.doc::<TantivyDocument>(doc_address)?;
            docs.push(doc_to_json(&schema, &doc));
        }
        Ok(docs)
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))?
}
