use std::path::{Path, PathBuf, Component};
use std::sync::Arc;

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
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

/// Validate that a user-supplied index name is safe to use as a filesystem path.
/// Rejects names containing path separators, parent directory references, or other
/// suspicious characters to prevent path traversal attacks.
pub fn validate_index_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(AppError::BadRequest("index name cannot be empty".to_string()));
    }
    if name.len() > 255 {
        return Err(AppError::BadRequest("index name too long (max 255 chars)".to_string()));
    }
    // Reject if the name would resolve to a path outside the base directory.
    let candidate = PathBuf::from(name);
    for component in candidate.components() {
        match component {
            Component::Normal(_) => {} // ok
            Component::ParentDir => {
                return Err(AppError::BadRequest(
                    "index name cannot contain '..'".to_string(),
                ));
            }
            Component::CurDir | Component::Prefix(_) | Component::RootDir => {
                return Err(AppError::BadRequest(
                    "index name contains invalid path components".to_string(),
                ));
            }
        }
    }
    // Also reject any name that contains characters commonly used in path attacks.
    if name.contains('/') || name.contains('\\') || name.contains('\0') {
        return Err(AppError::BadRequest(
            "index name contains invalid characters".to_string(),
        ));
    }
    Ok(())
}

/// Build the absolute path for an index directory and verify it stays within base_dir.
fn safe_index_path(base_dir: &Path, name: &str) -> Result<PathBuf> {
    validate_index_name(name)?;
    let path = base_dir.join(name);
    // Canonicalize base_dir and verify the resolved path is still under it.
    // We check the canonical form of the parent (base_dir) to handle symlinks.
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

/// Lock the managed writer, recovering from poison if necessary.
/// A poisoned mutex means a previous holder panicked; we can still use
/// the writer because Tantivy's IndexWriter state is not corrupted by
/// a panic in our wrapper code.
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

/// Tracks indexes that are currently being deleted to prevent TOCTOU races.
/// The DashMap key is the index name; the value exists only as a sentinel.
static DELETING: std::sync::LazyLock<DashMap<String, ()>> = std::sync::LazyLock::new(DashMap::new);

/// Manages multiple indexes dynamically.
#[derive(Clone)]
pub struct IndexManager {
    base_dir: PathBuf,
    indexes: DashMap<String, Arc<IndexHandle>>,
}

impl IndexManager {
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&base_dir)?;
        Ok(Self {
            base_dir,
            indexes: DashMap::new(),
        })
    }

    pub async fn create_index(
        &self,
        name: &str,
        schema_def: &SchemaDef,
    ) -> Result<Arc<IndexHandle>> {
        validate_index_name(name)?;

        match self.indexes.entry(name.to_string()) {
            Entry::Occupied(_) => Err(AppError::IndexAlreadyExists(name.to_string())),
            Entry::Vacant(entry) => {
                let index_dir = safe_index_path(&self.base_dir, name)?;
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
                    name: name.to_string(),
                    index,
                    schema: schema.clone(),
                    reader,
                    writer: managed,
                    expired_at_field: resolve_expired_at_field(&schema),
                });

                entry.insert(handle.clone());
                Ok(handle)
            }
        }
    }

    pub async fn open_index(&self, name: &str) -> Result<Arc<IndexHandle>> {
        validate_index_name(name)?;

        // Reject if this index is currently being deleted.
        if DELETING.contains_key(name) {
            return Err(AppError::IndexNotFound(name.to_string()));
        }

        match self.indexes.entry(name.to_string()) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(entry) => {
                let index_dir = safe_index_path(&self.base_dir, name)?;
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
                    name: name.to_string(),
                    index,
                    schema: schema.clone(),
                    reader,
                    writer: managed,
                    expired_at_field: resolve_expired_at_field(&schema),
                });

                entry.insert(handle.clone());
                Ok(handle)
            }
        }
    }

    pub async fn delete_index(&self, name: &str) -> Result<()> {
        validate_index_name(name)?;

        // Use a sentinel to prevent concurrent open/reopen during deletion.
        if DELETING.contains_key(name) {
            return Err(AppError::IndexNotFound(name.to_string()));
        }
        DELETING.insert(name.to_string(), ());

        // Ensure we always clean up the sentinel, even on error.
        let result = self.delete_index_inner(name).await;
        DELETING.remove(name);
        result
    }

    async fn delete_index_inner(&self, name: &str) -> Result<()> {
        // Remove from live map and drop the writer.
        if let Some((_, handle)) = self.indexes.remove(name) {
            let mut managed = lock_writer(&handle.writer);
            managed.writer.take(); // drop IndexWriter, releasing file locks
            drop(managed);
            drop(handle);
        }

        let index_dir = safe_index_path(&self.base_dir, name)?;
        if index_dir.exists() {
            tokio::fs::remove_dir_all(&index_dir).await?;
        }
        Ok(())
    }

    pub fn list_indexes(&self) -> Vec<String> {
        self.indexes.iter().map(|e| e.key().clone()).collect()
    }

    pub fn iter_handles(&self) -> Vec<Arc<IndexHandle>> {
        self.indexes.iter().map(|e| e.value().clone()).collect()
    }

    pub fn load_all_indexes(&self) -> Result<Vec<String>> {
        let mut loaded = Vec::new();
        for entry in std::fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let name = match entry.file_name().into_string() {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                // Skip indexes that are being deleted.
                if DELETING.contains_key(&name) {
                    continue;
                }
                // Validate the directory name is safe.
                if validate_index_name(&name).is_err() {
                    tracing::warn!(name = %name, "skipping index with invalid name");
                    continue;
                }
                if let Entry::Vacant(vacant) = self.indexes.entry(name.clone()) {
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
                    vacant.insert(handle);
                    loaded.push(name);
                }
            }
        }
        Ok(loaded)
    }
}
