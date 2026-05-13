use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
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
#[allow(dead_code)]
pub enum IndexStatus {
    Idle,
    Rebuilding,
}

impl IndexStatus {
    pub const IDLE_U8: u8 = 0;
    pub const REBUILDING_U8: u8 = 1;
}

/// Manages the lifecycle of an IndexWriter slot.
pub struct WriterSlot {
    pub writer: Option<IndexWriter>,
}

/// Holds an opened index together with its shared writer lock and reader.
pub struct IndexHandle {
    pub name: String,
    pub index: Index,
    pub schema: Schema,
    pub reader: IndexReader,
    /// Shared writer slot.
    pub writer: Arc<RwLock<WriterSlot>>,
    /// Whether the index has uncommitted writes.
    pub dirty: Arc<AtomicBool>,
    /// If the schema contains an `expired_at` date field, document expiration is enabled.
    pub expired_at_field: Option<Field>,
    /// Current lifecycle status of the index (0 = Idle, 1 = Rebuilding).
    pub status: Arc<AtomicU8>,
}

impl std::fmt::Debug for IndexHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexHandle")
            .field("name", &self.name)
            .field("schema", &self.schema)
            .field("expired_at_field", &self.expired_at_field)
            .field("dirty", &self.dirty.load(Ordering::Relaxed))
            .field("status", &self.status.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

/// Lifecycle state of an index within the manager.
#[derive(Clone)]
pub enum IndexState {
    Active(Arc<IndexHandle>),
    Deleting,
}
