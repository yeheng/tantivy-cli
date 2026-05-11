use std::ops::Bound;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tantivy::{
    DocAddress, TantivyDocument,
    query::{RangeQuery, TermQuery},
    schema::{IndexRecordOption, OwnedValue, Term},
};

use crate::error::{AppError, Result};
use crate::index::manager::IndexHandle;

/// Convert a JSON object into a Tantivy document based on the schema.
pub fn json_to_doc(schema: &tantivy::schema::Schema, data: &JsonValue) -> Result<TantivyDocument> {
    let obj = data
        .as_object()
        .ok_or_else(|| AppError::BadRequest("document must be a JSON object".to_string()))?;

    let mut doc = TantivyDocument::default();
    for (key, val) in obj {
        let field = schema
            .get_field(key)
            .map_err(|_| AppError::FieldNotFound(key.clone()))?;
        let field_entry = schema.get_field_entry(field);
        let owned_val = json_value_to_owned_value(val, field_entry.field_type())?;
        doc.add_field_value(field, &owned_val);
    }
    Ok(doc)
}

fn json_value_to_owned_value(
    value: &JsonValue,
    field_type: &tantivy::schema::FieldType,
) -> Result<OwnedValue> {
    match field_type {
        tantivy::schema::FieldType::Str(_) => Ok(OwnedValue::Str(
            value
                .as_str()
                .ok_or_else(|| AppError::Schema("expected string".to_string()))?
                .to_string(),
        )),
        tantivy::schema::FieldType::U64(_) => {
            Ok(OwnedValue::U64(value.as_u64().ok_or_else(|| {
                AppError::Schema("expected u64".to_string())
            })?))
        }
        tantivy::schema::FieldType::I64(_) => {
            Ok(OwnedValue::I64(value.as_i64().ok_or_else(|| {
                AppError::Schema("expected i64".to_string())
            })?))
        }
        tantivy::schema::FieldType::F64(_) => {
            Ok(OwnedValue::F64(value.as_f64().ok_or_else(|| {
                AppError::Schema("expected f64".to_string())
            })?))
        }
        tantivy::schema::FieldType::Bool(_) => {
            Ok(OwnedValue::Bool(value.as_bool().ok_or_else(|| {
                AppError::Schema("expected bool".to_string())
            })?))
        }
        tantivy::schema::FieldType::Date(_) => {
            let s = value
                .as_str()
                .ok_or_else(|| AppError::Schema("expected date string".to_string()))?;
            let dt = s
                .parse::<chrono::DateTime<chrono::Utc>>()
                .map_err(|e| AppError::Schema(format!("invalid date: {e}")))?;
            Ok(OwnedValue::Date(tantivy::DateTime::from_timestamp_secs(
                dt.timestamp(),
            )))
        }
        tantivy::schema::FieldType::Facet(_) => {
            let s = value
                .as_str()
                .ok_or_else(|| AppError::Schema("expected facet string".to_string()))?;
            Ok(OwnedValue::Facet(tantivy::schema::Facet::from_text(s)?))
        }
        tantivy::schema::FieldType::Bytes(_) => {
            let s = value
                .as_str()
                .ok_or_else(|| AppError::Schema("expected base64 bytes".to_string()))?;
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(s)
                .map_err(|e| AppError::Schema(format!("base64: {e}")))?;
            Ok(OwnedValue::Bytes(bytes))
        }
        tantivy::schema::FieldType::JsonObject(_) => {
            let map = value
                .as_object()
                .ok_or_else(|| AppError::Schema("expected json object".to_string()))?;
            let mut obj = Vec::new();
            for (k, v) in map {
                obj.push((k.clone(), json_to_owned_value(v)?));
            }
            Ok(OwnedValue::Object(obj))
        }
        _ => Err(AppError::Schema("unsupported field type".to_string())),
    }
}

fn json_to_owned_value(value: &JsonValue) -> Result<OwnedValue> {
    match value {
        JsonValue::Null => Ok(OwnedValue::Null),
        JsonValue::Bool(b) => Ok(OwnedValue::Bool(*b)),
        JsonValue::Number(n) => {
            if let Some(u) = n.as_u64() {
                Ok(OwnedValue::U64(u))
            } else if let Some(i) = n.as_i64() {
                Ok(OwnedValue::I64(i))
            } else if let Some(f) = n.as_f64() {
                Ok(OwnedValue::F64(f))
            } else {
                Err(AppError::Schema("invalid number".to_string()))
            }
        }
        JsonValue::String(s) => Ok(OwnedValue::Str(s.clone())),
        JsonValue::Array(arr) => {
            let mut vec = Vec::new();
            for v in arr {
                vec.push(json_to_owned_value(v)?);
            }
            Ok(OwnedValue::Array(vec))
        }
        JsonValue::Object(map) => {
            let mut obj = Vec::new();
            for (k, v) in map {
                obj.push((k.clone(), json_to_owned_value(v)?));
            }
            Ok(OwnedValue::Object(obj))
        }
    }
}

pub fn doc_to_json(schema: &tantivy::schema::Schema, doc: &TantivyDocument) -> JsonValue {
    let mut map = serde_json::Map::new();
    for (field, value) in doc.field_values() {
        let name = schema.get_field_name(field);
        let owned: OwnedValue = value.into();
        let json_val = match owned {
            OwnedValue::Str(s) => JsonValue::String(s),
            OwnedValue::U64(v) => JsonValue::Number(v.into()),
            OwnedValue::I64(v) => JsonValue::Number(v.into()),
            OwnedValue::F64(v) => serde_json::Number::from_f64(v)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null),
            OwnedValue::Bool(v) => JsonValue::Bool(v),
            OwnedValue::Date(v) => JsonValue::String(v.into_timestamp_secs().to_string()),
            OwnedValue::Facet(v) => JsonValue::String(v.to_string()),
            OwnedValue::Bytes(v) => {
                use base64::Engine;
                JsonValue::String(base64::engine::general_purpose::STANDARD.encode(v))
            }
            OwnedValue::Array(arr) => JsonValue::Array(
                arr.into_iter()
                    .map(|v| match v {
                        OwnedValue::Str(s) => JsonValue::String(s),
                        OwnedValue::U64(n) => JsonValue::Number(n.into()),
                        OwnedValue::I64(n) => JsonValue::Number(n.into()),
                        OwnedValue::F64(n) => serde_json::Number::from_f64(n)
                            .map(JsonValue::Number)
                            .unwrap_or(JsonValue::Null),
                        OwnedValue::Bool(b) => JsonValue::Bool(b),
                        _ => JsonValue::Null,
                    })
                    .collect(),
            ),
            OwnedValue::Object(obj) => serde_json::to_value(obj).unwrap_or(JsonValue::Null),
            _ => JsonValue::Null,
        };
        map.insert(name.to_string(), json_val);
    }
    JsonValue::Object(map)
}

/// Add or update a document (does NOT commit).
pub async fn add_document(handle: &IndexHandle, doc_json: &JsonValue) -> Result<String> {
    let doc = json_to_doc(&handle.schema, doc_json)?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    handle
        .writer_tx
        .send(crate::index::manager::IndexCommand::AddDoc(doc, tx))
        .await
        .map_err(|_| AppError::Internal("writer closed".to_string()))?;
    rx.await
        .map_err(|_| AppError::Internal("writer dropped".to_string()))??;

    let id = doc_json
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(id)
}

/// Delete documents by term query on a given field (does NOT commit).
pub async fn delete_documents(
    handle: &IndexHandle,
    field_name: &str,
    term_value: &str,
) -> Result<u64> {
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

    let (tx, rx) = tokio::sync::oneshot::channel();
    handle
        .writer_tx
        .send(crate::index::manager::IndexCommand::DeleteTerm(term, tx))
        .await
        .map_err(|_| AppError::Internal("writer closed".to_string()))?;
    rx.await
        .map_err(|_| AppError::Internal("writer dropped".to_string()))??;

    Ok(1)
}

/// Commit any pending changes for an index.
pub async fn commit_index(handle: &IndexHandle) -> Result<()> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    handle
        .writer_tx
        .send(crate::index::manager::IndexCommand::Commit(tx))
        .await
        .map_err(|_| AppError::Internal("writer closed".to_string()))?;
    rx.await
        .map_err(|_| AppError::Internal("writer dropped".to_string()))??;
    Ok(())
}

/// Get document by its internal doc address (segment_ord, doc_id) or by a unique id field.
pub async fn get_document(
    handle: &IndexHandle,
    field_name: Option<&str>,
    term_value: &str,
) -> Result<JsonValue> {
    let searcher = handle.reader.searcher();

    let doc = if let Some(field_name) = field_name {
        let field = handle
            .schema
            .get_field(field_name)
            .map_err(|_| AppError::FieldNotFound(field_name.to_string()))?;
        let field_entry = handle.schema.get_field_entry(field);
        let term = match field_entry.field_type() {
            tantivy::schema::FieldType::Str(_) => Term::from_field_text(field, term_value),
            tantivy::schema::FieldType::U64(_) => Term::from_field_u64(field, term_value.parse()?),
            tantivy::schema::FieldType::I64(_) => Term::from_field_i64(field, term_value.parse()?),
            _ => {
                return Err(AppError::Schema(
                    "unsupported lookup field type".to_string(),
                ));
            }
        };
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        let top_docs: Vec<(f32, tantivy::DocAddress)> = searcher.search(
            &query,
            &tantivy::collector::TopDocs::with_limit(1).order_by_score(),
        )?;
        if let Some((_, doc_address)) = top_docs.into_iter().next() {
            searcher.doc::<TantivyDocument>(doc_address)?
        } else {
            return Err(AppError::DocNotFound(term_value.to_string()));
        }
    } else {
        return Err(AppError::BadRequest(
            "field_name required for get".to_string(),
        ));
    };

    Ok(doc_to_json(&handle.schema, &doc))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexStats {
    pub num_docs: u64,
    pub num_segments: usize,
    pub schema: serde_json::Value,
}

pub async fn index_stats(handle: &IndexHandle) -> Result<IndexStats> {
    let searcher = handle.reader.searcher();
    let num_docs = searcher.num_docs();
    let num_segments = searcher.segment_readers().len();
    let schema_json = serde_json::to_value(&handle.schema)?;

    Ok(IndexStats {
        num_docs,
        num_segments,
        schema: schema_json,
    })
}

/// Rebuild the index by committing and merging all segments into one.
pub async fn rebuild_index(handle: &IndexHandle) -> Result<()> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    handle
        .writer_tx
        .send(crate::index::manager::IndexCommand::Rebuild(tx))
        .await
        .map_err(|_| AppError::Internal("writer closed".to_string()))?;
    rx.await
        .map_err(|_| AppError::Internal("writer dropped".to_string()))??;
    Ok(())
}

/// Compress (commit) the index.
pub async fn compress_index(handle: &IndexHandle) -> Result<()> {
    commit_index(handle).await
}

/// Delete documents whose `expired_at` timestamp is earlier than now.
/// If the index does not have an `expired_at` date field, this is a no-op.
pub async fn cleanup_expired(handle: &IndexHandle) -> Result<()> {
    let field = match handle.expired_at_field {
        Some(f) => f,
        None => return Ok(()),
    };

    let now = tantivy::DateTime::from_timestamp_secs(chrono::Utc::now().timestamp());
    let upper = Bound::Excluded(Term::from_field_date_for_search(field, now));
    let query = RangeQuery::new(Bound::Unbounded, upper);

    let (tx, rx) = tokio::sync::oneshot::channel();
    handle
        .writer_tx
        .send(crate::index::manager::IndexCommand::CleanupExpired(
            Box::new(query),
            tx,
        ))
        .await
        .map_err(|_| AppError::Internal("writer closed".to_string()))?;
    rx.await
        .map_err(|_| AppError::Internal("writer dropped".to_string()))??;

    tracing::info!(index = %handle.name, "cleaned up expired documents");
    Ok(())
}

/// List all documents (paginated).
/// Uses a hard limit of 10_000 on offset+limit to prevent OOM on deep pagination.
const MAX_RESULT_WINDOW: usize = 10_000;

pub async fn list_documents(
    handle: &IndexHandle,
    limit: usize,
    offset: usize,
) -> Result<Vec<JsonValue>> {
    if offset + limit > MAX_RESULT_WINDOW {
        return Err(AppError::BadRequest(format!(
            "offset + limit cannot exceed {MAX_RESULT_WINDOW}"
        )));
    }

    let searcher = handle.reader.searcher();
    let mut docs = Vec::with_capacity(limit);
    let mut seen = 0usize;

    for (segment_ord, segment_reader) in searcher.segment_readers().iter().enumerate() {
        if docs.len() >= limit {
            break;
        }
        for doc_id in segment_reader.doc_ids_alive() {
            if docs.len() >= limit {
                break;
            }
            if seen < offset {
                seen += 1;
                continue;
            }
            let doc =
                searcher.doc::<TantivyDocument>(DocAddress::new(segment_ord as u32, doc_id))?;
            docs.push(doc_to_json(&handle.schema, &doc));
            seen += 1;
        }
    }
    Ok(docs)
}
