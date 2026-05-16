use std::ops::Bound;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tantivy::{
    query::RangeQuery,
    schema::Term,
};

use crate::error::{AppError, Result};
use crate::index::handle::IndexStatus;
use crate::index::manager::IndexHandle;


#[derive(Debug, Serialize, Deserialize)]
pub struct IndexStats {
    pub num_docs: u64,
    pub num_segments: usize,
    pub schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

pub async fn index_stats(handle: Arc<IndexHandle>) -> Result<IndexStats> {
    let reader = handle.reader.clone();
    let schema_def = handle.schema_def.clone();
    tokio::task::spawn_blocking(move || {
        let searcher = reader.searcher();
        let num_docs = searcher.num_docs();
        let num_segments = searcher.segment_readers().len();
        let schema_json = serde_json::to_value(&schema_def)?;
        Ok(IndexStats {
            num_docs,
            num_segments,
            schema: schema_json,
            status: None,
        })
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))?
}

struct RebuildGuard(Arc<IndexHandle>);

impl Drop for RebuildGuard {
    fn drop(&mut self) {
        self.0.status.store(IndexStatus::Idle as u8, Ordering::Release);
    }
}

/// Fire-and-forget rebuild: sets the index status to Rebuilding and spawns
/// a background task that runs the actual merge. The status is reset to Idle
/// via an RAII guard when the task completes or panics.
pub fn trigger_rebuild(handle: Arc<IndexHandle>) -> Result<()> {
    let prev = handle.status.compare_exchange(
        IndexStatus::Idle as u8,
        IndexStatus::Rebuilding as u8,
        Ordering::AcqRel,
        Ordering::Acquire,
    );
    if prev.is_err() {
        return Err(AppError::Conflict("Index is already rebuilding".to_string()));
    }

    tokio::spawn(async move {
        let _guard = RebuildGuard(Arc::clone(&handle));
        if let Err(e) = rebuild_index(handle).await {
            tracing::error!(index = %_guard.0.name, error = %e, "rebuild failed");
        }
    });

    Ok(())
}

/// Rebuild the index by committing and merging all segments into one.
/// The lock is released between commit and merge so other writes are not
/// blocked for the entire duration of the operation.
pub async fn rebuild_index(handle: Arc<IndexHandle>) -> Result<()> {
    // Phase 1: commit under lock (fast).
    {
        let handle = Arc::clone(&handle);
        tokio::task::spawn_blocking(move || {
            let mut w = handle.writer.write().unwrap();
            if handle.dirty.load(Ordering::Acquire) {
                let writer = w
                    .as_mut()
                    .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?;
                writer.commit()?;
                handle.dirty.store(false, Ordering::Release);
            }
            Ok::<(), AppError>(())
        })
        .await
        .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))??;
    }

    // Phase 2+3: collect segment ids and trigger merge under lock,
    // then drop lock before waiting for merge to complete.
    let handle = Arc::clone(&handle);
    tokio::task::spawn_blocking(move || {
        let merge_result = {
            let mut w = handle.writer.write().unwrap();
            let writer = w
                .as_mut()
                .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?;

            // Collect segments under the same lock to avoid stale IDs
            // from concurrent commits between collection and merge.
            let segments = handle.index.searchable_segments()?;
            let segment_ids: Vec<_> = segments.iter().map(|s| s.id()).collect();
            if segment_ids.len() <= 1 {
                return Ok(());
            }

            writer.merge(&segment_ids)
        }; // Lock is dropped here!
        merge_result.wait().map_err(|e| AppError::Internal(e.to_string()))?;

        // Commit to make the merge durable.
        let mut w = handle.writer.write().unwrap();
        let writer = w
            .as_mut()
            .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?;
        writer.commit()?;
        Ok::<(), AppError>(())
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))??;

    Ok(())
}

/// Commit and wait for merge to complete, then commit again.
/// This is a real "compress" operation — not just a commit alias.
pub async fn compress_index(handle: Arc<IndexHandle>) -> Result<()> {
    rebuild_index(handle).await
}

/// Delete documents whose `expired_at` timestamp is earlier than now.
/// If the index does not have an `expired_at` date field, this is a no-op.
pub async fn cleanup_expired(handle: Arc<IndexHandle>) -> Result<()> {
    let field = match handle.expired_at_field {
        Some(f) => f,
        None => return Ok(()),
    };

    let now = tantivy::DateTime::from_timestamp_micros(chrono::Utc::now().timestamp_micros());
    let upper = Bound::Excluded(Term::from_field_date_for_search(field, now));

    let handle = Arc::clone(&handle);
    tokio::task::spawn_blocking(move || {
        let w = handle.writer.read().unwrap();
        w.as_ref()
            .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?
            .delete_query(Box::new(RangeQuery::new(Bound::Unbounded, upper)))?;
        handle.dirty.store(true, Ordering::Release);
        tracing::info!(index = %handle.name, "expired document cleanup attempted");
        Ok::<(), AppError>(())
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))??;

    Ok(())
}
