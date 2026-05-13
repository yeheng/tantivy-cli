use std::path::Path;
use std::sync::Arc;

use dashmap::DashMap;
use tantivy::schema::FieldType;
use tantivy::{Index, IndexBuilder, ReloadPolicy, TantivyDocument};

use crate::error::{AppError, Result};
use crate::index::schema::SchemaDef;

const WRITER_HEAP_BYTES: usize = 15_000_000;

pub use crate::index::handle::{IndexHandle, IndexState, IndexStatus, WriterSlot};
pub use crate::index::ops::validate_index_name;

/// Manages multiple indexes dynamically.
#[derive(Clone)]
pub struct IndexManager {
    base_dir: std::path::PathBuf,
    indexes: Arc<DashMap<String, IndexState>>,
    open_lock: Arc<tokio::sync::Mutex<()>>,
}

impl IndexManager {
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&base_dir)?;
        Ok(Self {
            base_dir,
            indexes: Arc::new(DashMap::new()),
            open_lock: Arc::new(tokio::sync::Mutex::new(())),
        })
    }

    pub async fn create_index(
        &self,
        name: &str,
        schema_def: &SchemaDef,
    ) -> Result<Arc<IndexHandle>> {
        validate_index_name(name)?;

        if self.indexes.contains_key(name) {
            return Err(AppError::IndexAlreadyExists(name.to_string()));
        }

        let base_dir = self.base_dir.clone();
        let name = name.to_string();
        let name_for_insert = name.clone();
        let schema_def = schema_def.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let index_dir = crate::index::ops::safe_index_path(&base_dir, &name)?;
            if index_dir.exists() {
                return Err(AppError::IndexAlreadyExists(name.to_string()));
            }
            std::fs::create_dir_all(&index_dir)?;

            let schema = schema_def.to_schema()?;
            let index = IndexBuilder::new()
                .schema(schema.clone())
                .create_in_dir(&index_dir)?;

            let reader = index
                .reader_builder()
                .reload_policy(ReloadPolicy::OnCommitWithDelay)
                .try_into()?;

            let writer = index.writer::<TantivyDocument>(WRITER_HEAP_BYTES)?;
            let text_fields: Vec<_> = schema
                .fields()
                .filter(|(_, entry)| matches!(entry.field_type(), FieldType::Str(_)))
                .map(|(field, _)| field)
                .collect();
            let handle = Arc::new(IndexHandle {
                name: name.clone(),
                index,
                schema: schema.clone(),
                reader,
                writer: Arc::new(std::sync::RwLock::new(WriterSlot {
                    writer: Some(writer),
                })),
                dirty: std::sync::atomic::AtomicBool::new(false),
                expired_at_field: crate::index::handle::resolve_expired_at_field(&schema),
                status: std::sync::atomic::AtomicU8::new(IndexStatus::Idle as u8),
                text_fields,
            });

            Ok::<Arc<IndexHandle>, AppError>(handle)
        })
        .await
        .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))??;

        match self.indexes.insert(name_for_insert.clone(), IndexState::Active(handle.clone())) {
            None => Ok(handle),
            Some(_) => Err(AppError::IndexAlreadyExists(name_for_insert)),
        }
    }

    pub async fn open_index(&self, name: &str) -> Result<Arc<IndexHandle>> {
        validate_index_name(name)?;

        // Fast path
        if let Some(state) = self.indexes.get(name) {
            return match state.value() {
                IndexState::Active(handle) => Ok(handle.clone()),
                IndexState::Deleting => Err(AppError::IndexNotFound(name.to_string())),
            };
        }

        // Slow path: serialize index initialization with a global lock.
        let _guard = self.open_lock.lock().await;

        // Double-checked locking
        if let Some(state) = self.indexes.get(name) {
            return match state.value() {
                IndexState::Active(handle) => Ok(handle.clone()),
                IndexState::Deleting => Err(AppError::IndexNotFound(name.to_string())),
            };
        }

        let base_dir = self.base_dir.clone();
        let name = name.to_string();
        let name_for_insert = name.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let index_dir = crate::index::ops::safe_index_path(&base_dir, &name)?;
            if !index_dir.exists() {
                return Err(AppError::IndexNotFound(name.to_string()));
            }

            let index = Index::open_in_dir(&index_dir)?;
            let schema = index.schema();
            let reader = index
                .reader_builder()
                .reload_policy(ReloadPolicy::OnCommitWithDelay)
                .try_into()?;

            let writer = index.writer::<TantivyDocument>(WRITER_HEAP_BYTES)?;
            let text_fields: Vec<_> = schema
                .fields()
                .filter(|(_, entry)| matches!(entry.field_type(), FieldType::Str(_)))
                .map(|(field, _)| field)
                .collect();
            let handle = Arc::new(IndexHandle {
                name: name.clone(),
                index,
                schema: schema.clone(),
                reader,
                writer: Arc::new(std::sync::RwLock::new(WriterSlot {
                    writer: Some(writer),
                })),
                dirty: std::sync::atomic::AtomicBool::new(false),
                expired_at_field: crate::index::handle::resolve_expired_at_field(&schema),
                status: std::sync::atomic::AtomicU8::new(IndexStatus::Idle as u8),
                text_fields,
            });

            Ok::<Arc<IndexHandle>, AppError>(handle)
        })
        .await
        .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))??;

        match self.indexes.insert(name_for_insert.clone(), IndexState::Active(handle.clone())) {
            None => Ok(handle),
            Some(IndexState::Active(existing)) => Ok(existing),
            Some(IndexState::Deleting) => {
                drop(handle);
                Err(AppError::IndexNotFound(name_for_insert))
            }
        }
    }

    pub async fn delete_index(&self, name: &str) -> Result<()> {
        validate_index_name(name)?;

        let mut handle = None;
        if let Some(mut entry) = self.indexes.get_mut(name) {
            if matches!(entry.value(), IndexState::Deleting) {
                return Err(AppError::IndexNotFound(name.to_string()));
            }
            let old = std::mem::replace(entry.value_mut(), IndexState::Deleting);
            if let IndexState::Active(h) = old {
                handle = Some(h);
            }
        } else {
            self.indexes.insert(name.to_string(), IndexState::Deleting);
        }

        let base_dir = self.base_dir.clone();
        let name = name.to_string();
        let name_for_remove = name.clone();
        let delete_result = tokio::task::spawn_blocking(move || {
            let index_dir = crate::index::ops::safe_index_path(&base_dir, &name)?;

            // Rename directory first so the original name is immediately reusable.
            let trash_dir = base_dir.join(format!(".deleting_{}_{}", name, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()));

            if let Some(handle) = handle {
                // 1. Release writer lock first, BEFORE any filesystem operations.
                let mut w = handle.writer.write().unwrap();
                w.writer.take(); // drop IndexWriter, releasing file locks
                drop(w);

                // 2. Now safe to rename directory (writer no longer holds .tantivy-meta.lock).
                if index_dir.exists() {
                    std::fs::rename(&index_dir, &trash_dir)?;
                }

                // 3. Wait for other Arc references to drop.
                let start = std::time::Instant::now();
                while Arc::strong_count(&handle) > 1 {
                    if start.elapsed() > std::time::Duration::from_secs(5) {
                        tracing::warn!(index = %name, "timeout waiting for index references to drop; leaving trash directory");
                        return Ok(());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                // 4. CRITICAL: drop handle to release reader and its mmap mappings.
                drop(handle);

                // 5. Now safe to physically delete directory.
                if trash_dir.exists() {
                    std::fs::remove_dir_all(&trash_dir)?;
                }
            } else {
                // No active handle: just rename and delete.
                if index_dir.exists() {
                    std::fs::rename(&index_dir, &trash_dir)?;
                }
                if trash_dir.exists() {
                    std::fs::remove_dir_all(&trash_dir)?;
                }
            }
            Ok::<(), AppError>(())
        })
        .await;

        match delete_result {
            Ok(Ok(())) => {
                self.indexes.remove(&name_for_remove);
                Ok(())
            }
            Ok(Err(e)) => {
                self.indexes.remove(&name_for_remove);
                Err(e)
            }
            Err(join_err) => {
                self.indexes.remove(&name_for_remove);
                Err(AppError::Internal(format!(
                    "spawn_blocking panicked: {join_err}"
                )))
            }
        }
    }

    pub fn list_indexes(&self) -> Vec<String> {
        self.indexes
            .iter()
            .filter_map(|e| match e.value() {
                IndexState::Active(_) => Some(e.key().clone()),
                IndexState::Deleting => None,
            })
            .collect()
    }

    pub fn iter_handles(&self) -> Vec<Arc<IndexHandle>> {
        self.indexes
            .iter()
            .filter_map(|e| match e.value() {
                IndexState::Active(h) => Some(h.clone()),
                IndexState::Deleting => None,
            })
            .collect()
    }

    pub async fn load_all_indexes(&self) -> Result<Vec<String>> {
        let base_dir = self.base_dir.clone();
        let handles = tokio::task::spawn_blocking(move || {
            let mut handles = Vec::new();
            for entry in std::fs::read_dir(&base_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let name = match entry.file_name().into_string() {
                        Ok(n) => n,
                        Err(_) => continue,
                    };
                    if validate_index_name(&name).is_err() {
                        tracing::warn!(name = %name, "skipping index with invalid name");
                        continue;
                    }
                    let index = match Index::open_in_dir(&path) {
                        Ok(i) => i,
                        Err(e) => {
                            tracing::warn!(index = %name, error = %e, "failed to open index");
                            continue;
                        }
                    };
                    let schema = index.schema();
                    let reader = match index
                        .reader_builder()
                        .reload_policy(ReloadPolicy::OnCommitWithDelay)
                        .try_into()
                    {
                        Ok(r) => r,
                        Err(e) => {
                            tracing::warn!(index = %name, error = %e, "failed to build reader");
                            continue;
                        }
                    };
                    let writer = match index.writer::<TantivyDocument>(WRITER_HEAP_BYTES) {
                        Ok(w) => w,
                        Err(e) => {
                            tracing::warn!(index = %name, error = %e, "failed to create writer");
                            continue;
                        }
                    };
                    let text_fields: Vec<_> = schema
                        .fields()
                        .filter(|(_, entry)| matches!(entry.field_type(), FieldType::Str(_)))
                        .map(|(field, _)| field)
                        .collect();
                    let handle = Arc::new(IndexHandle {
                        name: name.clone(),
                        index,
                        schema: schema.clone(),
                        reader,
                        writer: Arc::new(std::sync::RwLock::new(WriterSlot {
                            writer: Some(writer),
                        })),
                        dirty: std::sync::atomic::AtomicBool::new(false),
                        expired_at_field: crate::index::handle::resolve_expired_at_field(&schema),
                        status: std::sync::atomic::AtomicU8::new(IndexStatus::Idle as u8),
                        text_fields,
                    });
                    handles.push((name, handle));
                }
            }
            Ok::<Vec<(String, Arc<IndexHandle>)>, AppError>(handles)
        })
        .await
        .map_err(|e| AppError::Internal(format!("spawn_blocking failed: {e}")))??;

        let mut loaded = Vec::new();
        for (name, handle) in handles {
            if self.indexes.contains_key(&name) {
                continue;
            }
            if self.indexes.insert(name.clone(), IndexState::Active(handle)).is_none() {
                loaded.push(name);
            }
        }
        Ok(loaded)
    }
}


#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::task::JoinSet;

    use crate::index::manager::IndexManager;
    use crate::index::ops::validate_index_name;
    use crate::index::schema::{FieldDef, FieldKind, SchemaDef};

    fn test_schema() -> SchemaDef {
        SchemaDef {
            fields: vec![
                FieldDef {
                    name: "title".to_string(),
                    kind: FieldKind::Text,
                    stored: true,
                    indexed: true,
                    fast: false,
                },
                FieldDef {
                    name: "count".to_string(),
                    kind: FieldKind::U64,
                    stored: true,
                    indexed: true,
                    fast: false,
                },
            ],
        }
    }

    #[tokio::test]
    async fn test_validate_index_name_whitelist() {
        assert!(validate_index_name("valid_name-123").is_ok());
        assert!(validate_index_name("").is_err());
        assert!(validate_index_name("a/b").is_err());
        assert!(validate_index_name("..").is_err());
        assert!(validate_index_name("test\\foo").is_err());
        assert!(validate_index_name(".hidden").is_err());
        assert!(validate_index_name("a\0b").is_err());
        assert!(validate_index_name(&"a".repeat(256)).is_err());
    }

    #[tokio::test]
    async fn test_manager_create_open_delete_lifecycle() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(tmp.path()).unwrap();
        let schema = test_schema();

        let handle = manager.create_index("test", &schema).await.unwrap();
        assert_eq!(handle.name, "test");

        let handle2 = manager.open_index("test").await.unwrap();
        assert!(Arc::ptr_eq(&handle, &handle2));

        manager.delete_index("test").await.unwrap();
        assert!(manager.open_index("test").await.is_err());

        // Re-create after delete should succeed
        manager.create_index("test", &schema).await.unwrap();
    }

    #[tokio::test]
    async fn test_open_index_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(tmp.path()).unwrap();
        let schema = test_schema();

        manager.create_index("test", &schema).await.unwrap();

        // Multiple sequential opens must return the exact same handle.
        let h1 = manager.open_index("test").await.unwrap();
        let h2 = manager.open_index("test").await.unwrap();
        assert!(Arc::ptr_eq(&h1, &h2));
        assert_eq!(Arc::as_ptr(&h1.writer), Arc::as_ptr(&h2.writer));
    }

    #[tokio::test]
    async fn test_delete_index_cleanup() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(tmp.path()).unwrap();
        let schema = test_schema();

        manager.create_index("test", &schema).await.unwrap();
        manager.delete_index("test").await.unwrap();

        // After deletion, the name should be fully reusable.
        assert!(manager.open_index("test").await.is_err());
        manager.create_index("test", &schema).await.unwrap();
    }

    #[tokio::test]
    async fn test_multi_instance_isolation() {
        let tmp_a = tempfile::tempdir().unwrap();
        let tmp_b = tempfile::tempdir().unwrap();
        let mgr_a = IndexManager::new(tmp_a.path()).unwrap();
        let mgr_b = IndexManager::new(tmp_b.path()).unwrap();
        let schema = test_schema();

        mgr_a.create_index("shared", &schema).await.unwrap();
        mgr_b.create_index("shared", &schema).await.unwrap();

        // Delete in A must not affect B
        mgr_a.delete_index("shared").await.unwrap();
        assert!(mgr_a.open_index("shared").await.is_err());
        assert!(mgr_b.open_index("shared").await.is_ok());
    }

    #[tokio::test]
    async fn test_concurrent_open_index_serializes_writer() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = std::sync::Arc::new(IndexManager::new(tmp.path()).unwrap());
        let schema = test_schema();

        manager.create_index("test", &schema).await.unwrap();
        // Evict from memory map to force reloads
        manager.indexes.remove("test");

        let mut set = JoinSet::new();
        for _ in 0..5 {
            let mgr = std::sync::Arc::clone(&manager);
            set.spawn(async move { mgr.open_index("test").await });
        }

        let mut writer_ptrs = Vec::new();
        while let Some(res) = set.join_next().await {
            let handle = res.unwrap().unwrap();
            writer_ptrs.push(Arc::as_ptr(&handle.writer));
        }

        // All concurrent opens must succeed and share the exact same writer.
        assert_eq!(writer_ptrs.len(), 5, "all concurrent open_index calls must succeed");
        let first = writer_ptrs[0];
        for ptr in &writer_ptrs {
            assert_eq!(*ptr, first, "all successful opens must share the same writer");
        }

        // Subsequent opens should now hit the fast path and succeed.
        let handle = manager.open_index("test").await.unwrap();
        assert_eq!(Arc::as_ptr(&handle.writer), first);
    }
}
