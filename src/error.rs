use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Index not found: {0}")]
    IndexNotFound(String),

    #[error("Index already exists: {0}")]
    IndexAlreadyExists(String),

    #[error("Field not found: {0}")]
    FieldNotFound(String),

    #[error("Schema error: {0}")]
    Schema(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Document not found: {0}")]
    DocNotFound(String),

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parse error: {0}")]
    Parse(String),
}

impl From<std::num::ParseIntError> for AppError {
    fn from(e: std::num::ParseIntError) -> Self {
        AppError::Parse(e.to_string())
    }
}

impl From<std::num::ParseFloatError> for AppError {
    fn from(e: std::num::ParseFloatError) -> Self {
        AppError::Parse(e.to_string())
    }
}

impl From<std::str::ParseBoolError> for AppError {
    fn from(e: std::str::ParseBoolError) -> Self {
        AppError::Parse(e.to_string())
    }
}

impl From<tantivy::schema::FacetParseError> for AppError {
    fn from(e: tantivy::schema::FacetParseError) -> Self {
        AppError::Schema(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::IndexNotFound(_) | AppError::DocNotFound(_) | AppError::FieldNotFound(_) => {
                (StatusCode::NOT_FOUND, self.to_string())
            }
            AppError::IndexAlreadyExists(_) | AppError::Schema(_) | AppError::Query(_) => {
                (StatusCode::CONFLICT, self.to_string())
            }
            AppError::BadRequest(_) | AppError::Parse(_) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal error: {self}"),
            ),
        };
        let body = Json(json!({ "error": msg }));
        (status, body).into_response()
    }
}
