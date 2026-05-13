use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde_json::Value as JsonValue;
use crate::error::{AppError, Result};
use crate::index::doc::{json_to_doc, str_to_term};
use crate::index::manager::IndexHandle;

/// Add a document to the index (does NOT commit).
pub async fn add_document(handle: Arc<IndexHandle>, doc_json: &JsonValue) -> Result<String> {
    let doc = json_to_doc(&handle.schema, doc_json)?;
    let handle = Arc::clone(&handle);
    tokio::task::spawn_blocking(move || {
        let w = handle.writer.read().unwrap();
        let writer = w
            .writer
            .as_ref()
            .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?;
        writer.add_document(doc)?;
        handle.dirty.store(true, Ordering::Release);
        Ok::<(), AppError>(())
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))??;

    let id = doc_json
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("document must contain a string 'id' field".to_string()))?
        .to_string();
    Ok(id)
}

/// Add multiple documents in a single operation (does NOT commit).
/// Returns the number of documents successfully queued.
pub async fn add_documents(handle: Arc<IndexHandle>, docs_json: Vec<JsonValue>) -> Result<usize> {
    let schema = handle.schema.clone();
    let handle = Arc::clone(&handle);

    tokio::task::spawn_blocking(move || {
        let mut docs = Vec::with_capacity(docs_json.len());
        for doc_json in &docs_json {
            docs.push(json_to_doc(&schema, doc_json)?);
        }

        let w = handle.writer.read().unwrap();
        let writer = w
            .writer
            .as_ref()
            .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?;
        let mut count = 0usize;
        for doc in docs {
            writer.add_document(doc)?;
            count += 1;
        }
        handle.dirty.store(true, Ordering::Release);
        Ok::<usize, AppError>(count)
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))?
}

/// Delete documents by term query on a given field (does NOT commit).
pub async fn delete_documents(
    handle: Arc<IndexHandle>,
    field_name: &str,
    term_value: &str,
) -> Result<()> {
    let field = handle
        .schema
        .get_field(field_name)
        .map_err(|_| AppError::FieldNotFound(field_name.to_string()))?;

    let field_entry = handle.schema.get_field_entry(field);
    let term = str_to_term(field, field_entry.field_type(), term_value)?;

    let handle = Arc::clone(&handle);
    tokio::task::spawn_blocking(move || {
        let w = handle.writer.read().unwrap();
        let writer = w
            .writer
            .as_ref()
            .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?;
        writer.delete_term(term);
        handle.dirty.store(true, Ordering::Release);
        Ok::<(), AppError>(())
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))??;
    Ok(())
}

/// Commit any pending changes for an index.
pub async fn commit_index(handle: Arc<IndexHandle>) -> Result<()> {
    let handle = Arc::clone(&handle);
    tokio::task::spawn_blocking(move || {
        let mut w = handle.writer.write().unwrap();
        if handle.dirty.load(Ordering::Acquire) {
            let writer = w
                .writer
                .as_mut()
                .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?;
            writer.commit()?;
            handle.dirty.store(false, Ordering::Release);
        }
        Ok::<(), AppError>(())
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))??;
    Ok(())
}
