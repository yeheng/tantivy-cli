use std::path::{Path, PathBuf};

use crate::error::{AppError, Result};

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

/// Build the absolute path for an index directory.
/// `name` MUST have been validated by `validate_index_name` before calling this.
pub fn safe_index_path(base_dir: &Path, name: &str) -> Result<PathBuf> {
    Ok(base_dir.join(name))
}
