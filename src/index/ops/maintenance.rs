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
use crate::index::ops::commit_index;

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexStats {
    pub num_docs: u64,
    pub num_segments: usize,
    pub schema: serde_json::Value,
}

pub async fn index_stats(handle: &IndexHandle) -> Result<IndexStats> {
    let reader = handle.reader.clone();
    let schema = handle.schema.clone();
    tokio::task::spawn_blocking(move || {
        let searcher = reader.searcher();
        let num_docs = searcher.num_docs();
        let num_segments = searcher.segment_readers().len();
        let schema_json = serde_json::to_value(&schema)?;
        Ok(IndexStats {
            num_docs,
            num_segments,
            schema: schema_json,
        })
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))?
}

struct RebuildGuard(Arc<std::sync::atomic::AtomicU8>);

impl Drop for RebuildGuard {
    fn drop(&mut self) {
        self.0.store(IndexStatus::IDLE_U8, Ordering::Release);
    }
}

/// Fire-and-forget rebuild: sets the index status to Rebuilding and spawns
/// a background task that runs the actual merge. The status is reset to Idle
/// via an RAII guard when the task completes or panics.
pub fn trigger_rebuild(handle: Arc<IndexHandle>) -> Result<()> {
    let status = handle.status.load(Ordering::Acquire);
    if status == IndexStatus::REBUILDING_U8 {
        return Err(AppError::Conflict("Index is already rebuilding".to_string()));
    }
    handle.status.store(IndexStatus::REBUILDING_U8, Ordering::Release);

    tokio::spawn(async move {
        let _guard = RebuildGuard(handle.status.clone());
        if let Err(e) = rebuild_index(&handle).await {
            tracing::error!(index = %handle.name, error = %e, "rebuild failed");
        }
    });

    Ok(())
}

/// Rebuild the index by committing and merging all segments into one.
/// The lock is released between commit and merge so other writes are not
/// blocked for the entire duration of the operation.
pub async fn rebuild_index(handle: &IndexHandle) -> Result<()> {
    // Phase 1: commit under lock (fast).
    {
        let writer = handle.writer.clone();
        let dirty = handle.dirty.clone();
        tokio::task::spawn_blocking(move || {
            let mut w = writer.write().unwrap();
            if dirty.load(Ordering::Acquire) {
                let writer = w
                    .writer
                    .as_mut()
                    .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?;
                writer.commit()?;
                dirty.store(false, Ordering::Release);
            }
            Ok::<(), AppError>(())
        })
        .await
        .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))??;
    }

    // Phase 2: collect segment ids (read-only, no lock needed).
    let segments = {
        let index = handle.index.clone();
        tokio::task::spawn_blocking(move || index.searchable_segments())
            .await
            .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))?
    };
    let segment_ids: Vec<_> = segments?.iter().map(|s| s.id()).collect();
    if segment_ids.len() <= 1 {
        return Ok(());
    }

    // Phase 3: merge under lock (trigger merge, then drop lock before waiting).
    let writer = handle.writer.clone();
    tokio::task::spawn_blocking(move || {
        let merge_result = {
            let mut w = writer.write().unwrap();
            let writer = w
                .writer
                .as_mut()
                .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?;
            writer.merge(&segment_ids)
        }; // Lock is dropped here!
        merge_result.wait().map_err(|e| AppError::Internal(e.to_string()))?;

        // Commit to make the merge durable.
        let mut w = writer.write().unwrap();
        let writer = w
            .writer
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
pub async fn compress_index(handle: &IndexHandle) -> Result<()> {
    // Commit first to flush any pending writes.
    commit_index(handle).await?;

    // Then merge all segments into one and commit again.
    rebuild_index(handle).await?;

    Ok(())
}

/// Delete documents whose `expired_at` timestamp is earlier than now.
/// If the index does not have an `expired_at` date field, this is a no-op.
pub async fn cleanup_expired(handle: &IndexHandle) -> Result<()> {
    let field = match handle.expired_at_field {
        Some(f) => f,
        None => return Ok(()),
    };

    let now = tantivy::DateTime::from_timestamp_micros(chrono::Utc::now().timestamp_micros());
    let upper = Bound::Excluded(Term::from_field_date_for_search(field, now));

    let writer = handle.writer.clone();
    let reader = handle.reader.clone();
    let dirty = handle.dirty.clone();
    let index_name = handle.name.clone();

    tokio::task::spawn_blocking(move || {
        let searcher = reader.searcher();
        let count = searcher.search(
            &RangeQuery::new(Bound::Unbounded, upper.clone()),
            &tantivy::collector::Count,
        )?;
        if count == 0 {
            return Ok(());
        }

        let w = writer.read().unwrap();
        w.writer
            .as_ref()
            .ok_or_else(|| AppError::Internal("writer unavailable".to_string()))?
            .delete_query(Box::new(RangeQuery::new(Bound::Unbounded, upper)))?;
        dirty.store(true, Ordering::Release);
        tracing::info!(index = %index_name, "cleaned up expired documents");
        Ok::<(), AppError>(())
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))??;

    Ok(())
}
