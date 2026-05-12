use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tantivy::schema::{Field, FieldType, Schema};
use tantivy::{Index, IndexReader, IndexWriter};
use std::sync::RwLock;

pub fn resolve_expired_at_field(schema: &Schema) -> Option<Field> {
    schema.get_field("expired_at").ok().and_then(|field| {
        let entry = schema.get_field_entry(field);
        if matches!(entry.field_type(), FieldType::Date(_)) {
            Some(field)
        } else {
            None
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexStatus {
    Idle,
    Rebuilding,
}

pub struct ManagedWriter {
    pub writer: Option<IndexWriter>,
    pub dirty: AtomicBool,
}

impl std::fmt::Debug for ManagedWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedWriter")
            .field("has_writer", &self.writer.is_some())
            .field("dirty", &self.dirty.load(Ordering::Relaxed))
            .finish()
    }
}

/// Holds an opened index together with its shared writer lock and reader.
pub struct IndexHandle {
    pub name: String,
    pub index: Index,
    pub schema: Schema,
    pub reader: IndexReader,
    /// Shared RwLock-protected writer.
    pub writer: Arc<RwLock<ManagedWriter>>,
    /// If the schema contains an `expired_at` date field, document expiration is enabled.
    pub expired_at_field: Option<Field>,
    /// Current lifecycle status of the index (e.g., Idle, Rebuilding).
    pub status: Arc<RwLock<IndexStatus>>,
}

impl std::fmt::Debug for IndexHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexHandle")
            .field("name", &self.name)
            .field("schema", &self.schema)
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
