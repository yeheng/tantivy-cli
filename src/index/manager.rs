use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use tantivy::schema::{Field, FieldType, Schema};
use tantivy::{Index, IndexBuilder, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};

use crate::error::{AppError, Result};
use crate::index::schema::SchemaDef;

fn resolve_expired_at_field(schema: &Schema) -> Option<Field> {
    schema.get_field("expired_at").ok().and_then(|field| {
        let entry = schema.get_field_entry(field);
        if matches!(entry.field_type(), FieldType::Date(_)) {
            Some(field)
        } else {
            None
        }
    })
}

const WRITER_HEAP_BYTES: usize = 15_000_000;

/// Validate that an index name is a safe identifier.
/// Only ASCII alphanumeric characters, underscores, and hyphens are allowed.
pub fn validate_index_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(AppError::BadRequest("index name cannot be empty".to_string()));
    }
    if name.len() > 255 {
        return Err(AppError::BadRequest("index name too long (max 255 chars)".to_string()));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(AppError::BadRequest(
            "index name contains invalid characters (allowed: a-z, A-Z, 0-9, _, -)".to_string(),
        ));
    }
    Ok(())
}

/// Build the absolute path for an index directory and verify it stays within base_dir.
fn safe_index_path(base_dir: &Path, name: &str) -> Result<PathBuf> {
    validate_index_name(name)?;
    let path = base_dir.join(name);
    let canonical_base = base_dir.canonicalize().unwrap_or_else(|_| base_dir.to_path_buf());
    let resolved = canonical_base.join(name);
    if !resolved.starts_with(&canonical_base) {
        return Err(AppError::BadRequest("invalid index name".to_string()));
    }
    Ok(path)
}

pub struct ManagedWriter {
    pub writer: Option<IndexWriter>,
    pub dirty: bool,
}

impl std::fmt::Debug for ManagedWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedWriter")
            .field("has_writer", &self.writer.is_some())
            .field("dirty", &self.dirty)
            .finish()
    }
}

fn lock_writer(
    writer: &Arc<parking_lot::Mutex<ManagedWriter>>,
) -> parking_lot::MutexGuard<'_, ManagedWriter> {
    writer.lock()
}

/// Holds an opened index together with its shared writer mutex and reader.
pub struct IndexHandle {
    pub name: String,
    pub index: Index,
    pub schema: Schema,
    pub reader: IndexReader,
    /// Shared mutex-protected writer.
    pub writer: Arc<parking_lot::Mutex<ManagedWriter>>,
    /// If the schema contains an `expired_at` date field, document expiration is enabled.
    pub expired_at_field: Option<Field>,
}

impl std::fmt::Debug for IndexHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexHandle")
            .field("name", &self.name)
            .field("schema", &self.schema)
            .field("dirty", &self.writer.lock().dirty)
            .field("expired_at_field", &self.expired_at_field)
            .finish_non_exhaustive()
    }
}

/// Lifecycle state of an index within the manager.
#[derive(Clone)]
pub enum IndexState {
    Active(Arc<IndexHandle>),
    Deleting,
}

/// Manages multiple indexes dynamically.
#[derive(Clone)]
pub struct IndexManager {
    base_dir: PathBuf,
    indexes: Arc<DashMap<String, IndexState>>,
}

impl IndexManager {
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&base_dir)?;
        Ok(Self {
            base_dir,
            indexes: Arc::new(DashMap::new()),
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
            let index_dir = safe_index_path(&base_dir, &name)?;
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
            let managed = Arc::new(parking_lot::Mutex::new(ManagedWriter {
                writer: Some(writer),
                dirty: false,
            }));

            let handle = Arc::new(IndexHandle {
                name: name.clone(),
                index,
                schema: schema.clone(),
                reader,
                writer: managed,
                expired_at_field: resolve_expired_at_field(&schema),
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
            let index_dir = safe_index_path(&base_dir, &name)?;
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
            let managed = Arc::new(parking_lot::Mutex::new(ManagedWriter {
                writer: Some(writer),
                dirty: false,
            }));

            let handle = Arc::new(IndexHandle {
                name: name.clone(),
                index,
                schema: schema.clone(),
                reader,
                writer: managed,
                expired_at_field: resolve_expired_at_field(&schema),
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
            if let Some(handle) = handle {
                let mut managed = lock_writer(&handle.writer);
                managed.writer.take(); // drop IndexWriter, releasing file locks
                drop(managed);
                drop(handle);
            }

            let index_dir = safe_index_path(&base_dir, &name)?;
            if index_dir.exists() {
                std::fs::remove_dir_all(&index_dir)?;
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
                    let managed = Arc::new(parking_lot::Mutex::new(ManagedWriter {
                        writer: Some(writer),
                        dirty: false,
                    }));
                    let handle = Arc::new(IndexHandle {
                        name: name.clone(),
                        index,
                        schema: schema.clone(),
                        reader,
                        writer: managed,
                        expired_at_field: resolve_expired_at_field(&schema),
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

    use crate::index::manager::{IndexManager, validate_index_name};
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

        let mut successes = 0;
        let mut writer_ptrs = Vec::new();
        while let Some(res) = set.join_next().await {
            match res.unwrap() {
                Ok(handle) => {
                    successes += 1;
                    writer_ptrs.push(Arc::as_ptr(&handle.writer));
                }
                Err(_) => {
                    // LockBusy is acceptable for racing open_index calls
                }
            }
        }

        // At least one must succeed, and all successes must share the same writer.
        assert!(
            successes >= 1,
            "at least one concurrent open_index must succeed"
        );
        let first = writer_ptrs[0];
        for ptr in &writer_ptrs {
            assert_eq!(*ptr, first, "all successful opens must share the same writer");
        }

        // Subsequent opens should now hit the fast path and succeed.
        let handle = manager.open_index("test").await.unwrap();
        assert_eq!(Arc::as_ptr(&handle.writer), first);
    }
}
