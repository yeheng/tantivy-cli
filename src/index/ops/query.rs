use std::sync::Arc;

use serde_json::Value as JsonValue;
use tantivy::{
    DocAddress, DocId, SegmentReader, TantivyDocument,
    collector::{Collector, SegmentCollector},
    query::{TermQuery},
    schema::IndexRecordOption,
};

use crate::error::{AppError, Result};
use crate::index::doc::{doc_to_json, str_to_term};
use crate::index::manager::IndexHandle;

/// Collector that gathers doc addresses without score-based heap ordering.
/// Used for `list_documents` where `AllQuery` assigns every doc the same score,
/// making a score-sorted TopDocs a waste of CPU cycles.
struct ListDocsCollector {
    limit: usize,
}

impl Collector for ListDocsCollector {
    type Fruit = Vec<DocAddress>;
    type Child = ListDocsSegmentCollector;

    fn for_segment(
        &self,
        segment_local_id: u32,
        _segment: &SegmentReader,
    ) -> tantivy::Result<ListDocsSegmentCollector> {
        Ok(ListDocsSegmentCollector {
            segment_local_id,
            docs: Vec::new(),
        })
    }

    fn requires_scoring(&self) -> bool {
        false
    }

    fn merge_fruits(
        &self,
        segment_fruits: Vec<Vec<DocAddress>>,
    ) -> tantivy::Result<Vec<DocAddress>> {
        let mut all = Vec::new();
        for docs in segment_fruits {
            all.extend(docs);
        }
        all.truncate(self.limit);
        Ok(all)
    }
}

struct ListDocsSegmentCollector {
    segment_local_id: u32,
    docs: Vec<DocAddress>,
}

impl SegmentCollector for ListDocsSegmentCollector {
    type Fruit = Vec<DocAddress>;

    fn collect(&mut self, doc: DocId, _score: f32) {
        self.docs.push(DocAddress {
            segment_ord: self.segment_local_id,
            doc_id: doc,
        });
    }

    fn harvest(self) -> Vec<DocAddress> {
        self.docs
    }
}

/// Get document by field value.
pub async fn get_document(
    handle: Arc<IndexHandle>,
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
        let (_, doc_address) = top_docs.into_iter().next()
            .ok_or(AppError::DocNotFound(term_value))?;
        let doc = searcher.doc::<TantivyDocument>(doc_address)?;

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
    handle: Arc<IndexHandle>,
    limit: usize,
    offset: usize,
) -> Result<Vec<JsonValue>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let window = offset
        .checked_add(limit)
        .ok_or_else(|| AppError::BadRequest("offset + limit overflow".to_string()))?;
    if window > MAX_RESULT_WINDOW {
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

        let doc_addresses: Vec<DocAddress> = searcher.search(
            &tantivy::query::AllQuery,
            &ListDocsCollector { limit: offset + limit },
        )?;

        let mut docs = Vec::with_capacity(doc_addresses.len().saturating_sub(offset));
        for doc_address in &doc_addresses[offset..] {
            let doc = searcher.doc::<TantivyDocument>(*doc_address)?;
            docs.push(doc_to_json(&schema, &doc));
        }
        Ok(docs)
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))?
}
