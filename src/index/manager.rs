use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use tantivy::schema::{Field, FieldType, Schema};
use tantivy::{Index, IndexBuilder, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};
use tokio::sync::mpsc;

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

/// Commands sent to the per-index writer actor.
pub enum IndexCommand {
    AddDoc(TantivyDocument, tokio::sync::oneshot::Sender<Result<()>>),
    DeleteTerm(
        tantivy::schema::Term,
        tokio::sync::oneshot::Sender<Result<()>>,
    ),
    Commit(tokio::sync::oneshot::Sender<Result<()>>),
    Rebuild(tokio::sync::oneshot::Sender<Result<()>>),
    CleanupExpired(
        Box<dyn tantivy::query::Query + Send + Sync>,
        tokio::sync::oneshot::Sender<Result<()>>,
    ),
}

/// Holds an opened index together with its writer channel and reader.
pub struct IndexHandle {
    pub name: String,
    pub index: Index,
    pub schema: Schema,
    pub reader: IndexReader,
    /// Sender to the single writer actor for this index.
    pub writer_tx: mpsc::Sender<IndexCommand>,
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

    pub async fn create_index(
        &self,
        name: &str,
        schema_def: &SchemaDef,
    ) -> Result<Arc<IndexHandle>> {
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

        let writer_tx = spawn_writer_actor(index.clone());

        let handle = Arc::new(IndexHandle {
            name: name.to_string(),
            index,
            schema: schema.clone(),
            reader,
            writer_tx,
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

        let writer_tx = spawn_writer_actor(index.clone());

        let handle = Arc::new(IndexHandle {
            name: name.to_string(),
            index,
            schema: schema.clone(),
            reader,
            writer_tx,
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
                        let writer_tx = spawn_writer_actor(index.clone());
                        let handle = Arc::new(IndexHandle {
                            name: name.clone(),
                            index,
                            schema: schema.clone(),
                            reader,
                            writer_tx,
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

fn spawn_writer_actor(index: Index) -> mpsc::Sender<IndexCommand> {
    let (tx, mut rx) = mpsc::channel::<IndexCommand>(1024);
    tokio::spawn(async move {
        let mut writer: IndexWriter = match index.writer::<TantivyDocument>(15_000_000) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!(error = %e, "failed to create IndexWriter");
                return;
            }
        };
        let mut dirty = false;
        while let Some(cmd) = rx.recv().await {
            match cmd {
                IndexCommand::AddDoc(doc, reply) => {
                    let res = writer
                        .add_document(doc)
                        .map(|_| ())
                        .map_err(AppError::Tantivy);
                    if res.is_ok() {
                        dirty = true;
                    }
                    let _ = reply.send(res);
                }
                IndexCommand::DeleteTerm(term, reply) => {
                    writer.delete_term(term);
                    dirty = true;
                    let _ = reply.send(Ok(()));
                }
                IndexCommand::Commit(reply) => {
                    if dirty {
                        let res = writer.commit().map(|_| ()).map_err(AppError::Tantivy);
                        dirty = false;
                        let _ = reply.send(res);
                    } else {
                        let _ = reply.send(Ok(()));
                    }
                }
                IndexCommand::Rebuild(reply) => {
                    let res = (|| -> Result<()> {
                        writer.commit()?;
                        let segments = index.searchable_segments()?;
                        let segment_ids: Vec<_> = segments.iter().map(|s| s.id()).collect();
                        if segment_ids.len() > 1 {
                            let merge_result = writer.merge(&segment_ids);
                            merge_result.wait()?;
                        }
                        Ok(())
                    })();
                    dirty = false;
                    let _ = reply.send(res);
                }
                IndexCommand::CleanupExpired(query, reply) => {
                    let res = (|| -> Result<()> {
                        writer.delete_query(query)?;
                        writer.commit()?;
                        Ok(())
                    })();
                    dirty = false;
                    let _ = reply.send(res);
                }
            }
        }
        tracing::info!("writer actor shutting down");
    });
    tx
}
