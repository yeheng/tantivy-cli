use std::path::{Path, PathBuf};
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

/// Lock the managed writer, recovering from poison if necessary.
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
    indexes: DashMap<String, IndexState>,
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

                entry.insert(IndexState::Active(handle.clone()));
                Ok(handle)
            }
        }
    }

    pub async fn open_index(&self, name: &str) -> Result<Arc<IndexHandle>> {
        validate_index_name(name)?;

        match self.indexes.entry(name.to_string()) {
            Entry::Occupied(entry) => match entry.get() {
                IndexState::Active(handle) => Ok(handle.clone()),
                IndexState::Deleting => Err(AppError::IndexNotFound(name.to_string())),
            },
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

                entry.insert(IndexState::Active(handle.clone()));
                Ok(handle)
            }
        }
    }

    pub async fn delete_index(&self, name: &str) -> Result<()> {
        validate_index_name(name)?;

        let handle = match self.indexes.entry(name.to_string()) {
            Entry::Occupied(mut entry) => {
                if matches!(entry.get(), IndexState::Deleting) {
                    return Err(AppError::IndexNotFound(name.to_string()));
                }
                match entry.insert(IndexState::Deleting) {
                    IndexState::Active(h) => Some(h),
                    IndexState::Deleting => None,
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(IndexState::Deleting);
                None
            }
        };

        let result = async {
            if let Some(handle) = handle {
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
        .await;

        self.indexes.remove(name);
        result
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
                if validate_index_name(&name).is_err() {
                    tracing::warn!(name = %name, "skipping index with invalid name");
                    continue;
                }
                if self.indexes.contains_key(&name) {
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
                self.indexes.insert(name.clone(), IndexState::Active(handle));
                loaded.push(name);
            }
        }
        Ok(loaded)
    }
}
