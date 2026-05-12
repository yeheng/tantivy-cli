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

/// Build the absolute path for an index directory and verify it stays within base_dir.
pub fn safe_index_path(base_dir: &Path, name: &str) -> Result<PathBuf> {
    validate_index_name(name)?;
    let path = base_dir.join(name);
    let canonical_base = base_dir.canonicalize().unwrap_or_else(|_| base_dir.to_path_buf());
    let resolved = canonical_base.join(name);
    if !resolved.starts_with(&canonical_base) {
        return Err(AppError::BadRequest("invalid index name".to_string()));
    }
    Ok(path)
}
