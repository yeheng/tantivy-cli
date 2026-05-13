//! Facade module that re-exports all index operations.
//!
//! Individual responsibilities are split into sub-modules:
//! - `doc`: JSON ↔ Tantivy document conversion
//! - `write`: document ingestion, deletion, and commit
//! - `query`: single-document lookup and paginated listing
//! - `maintenance`: stats, rebuild, compress, cleanup

mod maintenance;
mod query;
mod validation;
mod write;

pub use crate::index::ops::maintenance::{
    cleanup_expired, compress_index, index_stats, rebuild_index, trigger_rebuild,
};
pub use crate::index::ops::query::{get_document, list_documents};
pub use crate::index::ops::validation::{safe_index_path, validate_index_name};
pub use crate::index::ops::write::{add_document, add_documents, commit_index, delete_documents};
