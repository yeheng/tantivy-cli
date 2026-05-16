use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use tantivy::schema::{Field, FieldType, Schema};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};
use std::sync::RwLock;

use crate::index::schema::SchemaDef;

const WRITER_HEAP_BYTES: usize = 15_000_000;

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
#[repr(u8)]
pub enum IndexStatus {
    Idle = 0,
    Rebuilding = 1,
}

impl IndexStatus {
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Idle),
            1 => Some(Self::Rebuilding),
            _ => None,
        }
    }
}

/// Holds an opened index together with its shared writer lock and reader.
pub struct IndexHandle {
    pub name: String,
    pub index: Index,
    pub schema: Schema,
    pub schema_def: SchemaDef,
    pub reader: IndexReader,
    /// Shared writer slot.
    pub writer: Arc<RwLock<Option<IndexWriter>>>,
    /// Whether the index has uncommitted writes.
    pub dirty: AtomicBool,
    /// If the schema contains an `expired_at` date field, document expiration is enabled.
    pub expired_at_field: Option<Field>,
    /// Current lifecycle status of the index (0 = Idle, 1 = Rebuilding).
    pub status: AtomicU8,
    /// Pre-computed list of text fields for QueryString searches.
    pub text_fields: Vec<Field>,
}

impl IndexHandle {
    pub fn build(name: String, index: Index, schema_def: SchemaDef) -> crate::error::Result<Arc<Self>> {
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
        Ok(Arc::new(IndexHandle {
            name: name.clone(),
            expired_at_field: resolve_expired_at_field(&schema),
            schema,
            schema_def,
            index,
            reader,
            writer: Arc::new(RwLock::new(Some(writer))),
            dirty: AtomicBool::new(false),
            status: AtomicU8::new(IndexStatus::Idle as u8),
            text_fields,
        }))
    }
}

impl std::fmt::Debug for IndexHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexHandle")
            .field("name", &self.name)
            .field("schema", &self.schema)
            .field("expired_at_field", &self.expired_at_field)
            .field("dirty", &self.dirty.load(Ordering::Relaxed))
            .field("status", &IndexStatus::from_u8(self.status.load(Ordering::Relaxed)))
            .finish_non_exhaustive()
    }
}

/// Lifecycle state of an index within the manager.
#[derive(Clone)]
pub enum IndexState {
    Active(Arc<IndexHandle>),
    Deleting,
}
