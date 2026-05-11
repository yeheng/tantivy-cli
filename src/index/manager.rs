use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use tantivy::schema::{Field, FieldType, Schema};
use tantivy::{Index, IndexBuilder, IndexReader, IndexWriter, ReloadPolicy};

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

/// Holds an opened index together with its writer and reader.
pub struct IndexHandle {
    pub name: String,
    pub index: Index,
    pub schema: Schema,
    pub reader: IndexReader,
    /// Writer is lazily initialized inside a std::sync::Mutex<Option<...>>.
    pub writer: Arc<std::sync::Mutex<Option<IndexWriter>>>,
    /// If the schema contains an `expired_at` date field, document expiration is enabled.
    pub expired_at_field: Option<Field>,
}

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

    pub async fn create_index(&self, name: &str, schema_def: &SchemaDef) -> Result<Arc<IndexHandle>> {
        if self.indexes.contains_key(name) {
            return Err(AppError::IndexAlreadyExists(name.to_string()));
        }

        let index_dir = self.base_dir.join(name);
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

        let handle = Arc::new(IndexHandle {
            name: name.to_string(),
            index,
            schema: schema.clone(),
            reader,
            writer: Arc::new(std::sync::Mutex::new(None)),
            expired_at_field: resolve_expired_at_field(&schema),
        });

        self.indexes.insert(name.to_string(), handle.clone());
        Ok(handle)
    }

    pub async fn open_index(&self, name: &str) -> Result<Arc<IndexHandle>> {
        if let Some(handle) = self.indexes.get(name) {
            return Ok(handle.clone());
        }

        let index_dir = self.base_dir.join(name);
        if !index_dir.exists() {
            return Err(AppError::IndexNotFound(name.to_string()));
        }

        let index = Index::open_in_dir(&index_dir)?;
        let schema = index.schema();
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let handle = Arc::new(IndexHandle {
            name: name.to_string(),
            index,
            schema: schema.clone(),
            reader,
            writer: Arc::new(std::sync::Mutex::new(None)),
            expired_at_field: resolve_expired_at_field(&schema),
        });

        self.indexes.insert(name.to_string(), handle.clone());
        Ok(handle)
    }

    pub async fn delete_index(&self, name: &str) -> Result<()> {
        self.indexes.remove(name);
        let index_dir = self.base_dir.join(name);
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
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                if !self.indexes.contains_key(&name) {
                    if let Ok(index) = Index::open_in_dir(&path) {
                        let schema = index.schema();
                        let reader = index
                            .reader_builder()
                            .reload_policy(ReloadPolicy::OnCommitWithDelay)
                            .try_into()?;
                        let handle = Arc::new(IndexHandle {
                            name: name.clone(),
                            index,
                            schema: schema.clone(),
                            reader,
                            writer: Arc::new(std::sync::Mutex::new(None)),
                            expired_at_field: resolve_expired_at_field(&schema),
                        });
                        self.indexes.insert(name.clone(), handle);
                        loaded.push(name);
                    }
                }
            }
        }
        Ok(loaded)
    }
}
