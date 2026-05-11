use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use tantivy::schema::Schema;
use tantivy::{Index, IndexBuilder, IndexReader, IndexWriter, ReloadPolicy};
use tokio::sync::RwLock;

use crate::error::{AppError, Result};
use crate::index::schema::SchemaDef;

/// Holds an opened index together with its writer and reader.
pub struct IndexHandle {
    pub name: String,
    pub index: Index,
    pub schema: Schema,
    pub reader: IndexReader,
    /// Writer is behind RwLock because tantivy::IndexWriter is not Send in some versions.
    pub writer: Arc<RwLock<IndexWriter>>,
    pub path: PathBuf,
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

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
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

        let writer = index.writer::<tantivy::TantivyDocument>(50_000_000)?;

        let handle = Arc::new(IndexHandle {
            name: name.to_string(),
            index,
            schema: schema.clone(),
            reader,
            writer: Arc::new(RwLock::new(writer)),
            path: index_dir,
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

        let writer = index.writer::<tantivy::TantivyDocument>(50_000_000)?;

        let handle = Arc::new(IndexHandle {
            name: name.to_string(),
            index,
            schema: schema.clone(),
            reader,
            writer: Arc::new(RwLock::new(writer)),
            path: index_dir,
        });

        self.indexes.insert(name.to_string(), handle.clone());
        Ok(handle)
    }

    pub fn get_index(&self, name: &str) -> Result<Arc<IndexHandle>> {
        self.indexes
            .get(name)
            .map(|h| h.clone())
            .ok_or_else(|| AppError::IndexNotFound(name.to_string()))
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

    pub fn index_exists(&self, name: &str) -> bool {
        self.base_dir.join(name).exists()
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
                        let writer = index.writer::<tantivy::TantivyDocument>(50_000_000)?;
                        let handle = Arc::new(IndexHandle {
                            name: name.clone(),
                            index,
                            schema,
                            reader,
                            writer: Arc::new(RwLock::new(writer)),
                            path,
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
