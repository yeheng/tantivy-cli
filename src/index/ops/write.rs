use std::sync::atomic::Ordering;

use serde_json::Value as JsonValue;
use tantivy::schema::Term;

use crate::error::{AppError, Result};
use crate::index::doc::json_to_doc;
use crate::index::manager::IndexHandle;

/// Add or update a document (does NOT commit).
pub async fn add_document(handle: &IndexHandle, doc_json: &JsonValue) -> Result<String> {
    let doc = json_to_doc(&handle.schema, doc_json)?;
    let writer = handle.writer.read().unwrap();
    let w = writer
        .writer
        .as_ref()
        .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?;
    w.add_document(doc)?;
    writer.dirty.store(true, Ordering::Release);
    drop(writer);

    let id = doc_json
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(id)
}

/// Add multiple documents in a single operation (does NOT commit).
/// Returns the number of documents successfully queued.
pub async fn add_documents(handle: &IndexHandle, docs_json: Vec<JsonValue>) -> Result<usize> {
    let schema = handle.schema.clone();
    let writer = handle.writer.clone();

    tokio::task::spawn_blocking(move || {
        let mut docs = Vec::with_capacity(docs_json.len());
        for doc_json in &docs_json {
            docs.push(json_to_doc(&schema, doc_json)?);
        }

        let writer = writer.read().unwrap();
        let w = writer
            .writer
            .as_ref()
            .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?;
        let mut count = 0usize;
        for doc in docs {
            w.add_document(doc)?;
            count += 1;
        }
        writer.dirty.store(true, Ordering::Release);
        Ok::<usize, AppError>(count)
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))?
}

/// Delete documents by term query on a given field (does NOT commit).
pub async fn delete_documents(
    handle: &IndexHandle,
    field_name: &str,
    term_value: &str,
) -> Result<()> {
    let field = handle
        .schema
        .get_field(field_name)
        .map_err(|_| AppError::FieldNotFound(field_name.to_string()))?;

    let field_entry = handle.schema.get_field_entry(field);
    let term = match field_entry.field_type() {
        tantivy::schema::FieldType::Str(_) => Term::from_field_text(field, term_value),
        tantivy::schema::FieldType::U64(_) => {
            let v = term_value.parse::<u64>()?;
            Term::from_field_u64(field, v)
        }
        tantivy::schema::FieldType::I64(_) => {
            let v = term_value.parse::<i64>()?;
            Term::from_field_i64(field, v)
        }
        tantivy::schema::FieldType::F64(_) => {
            let v = term_value.parse::<f64>()?;
            Term::from_field_f64(field, v)
        }
        tantivy::schema::FieldType::Bool(_) => {
            let v = term_value.parse::<bool>()?;
            Term::from_field_bool(field, v)
        }
        _ => {
            return Err(AppError::Schema(
                "unsupported delete field type".to_string(),
            ));
        }
    };

    let writer = handle.writer.read().unwrap();
    let w = writer
        .writer
        .as_ref()
        .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?;
    w.delete_term(term);
    writer.dirty.store(true, Ordering::Release);
    drop(writer);
    Ok(())
}

/// Commit any pending changes for an index.
pub async fn commit_index(handle: &IndexHandle) -> Result<()> {
    let writer = handle.writer.clone();
    tokio::task::spawn_blocking(move || {
        let mut managed = writer.write().unwrap();
        if managed.dirty.load(Ordering::Acquire) {
            managed
                .writer
                .as_mut()
                .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?
                .commit()?;
            managed.dirty.store(false, Ordering::Release);
        }
        Ok::<(), AppError>(())
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))??;
    Ok(())
}
