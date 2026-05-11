use actix_web::{HttpResponse, ResponseError};
use thiserror::Error;

use crate::repository::RepositoryError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("file storage error: {0}")]
    Storage(String),

    #[error("repository error: {0}")]
    Repository(#[from] RepositoryError),

    #[error("internal server error")]
    Internal,
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::BadRequest(message) => HttpResponse::BadRequest().body(message.to_string()),
            AppError::NotFound(message) => HttpResponse::NotFound().body(message.to_string()),
            AppError::Storage(message) => {
                HttpResponse::InternalServerError().body(message.to_string())
            }
            AppError::Repository(_) => HttpResponse::InternalServerError().body(self.to_string()),
            AppError::Internal => HttpResponse::InternalServerError().body(self.to_string()),
        }
    }
}
