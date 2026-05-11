pub mod sqlite_image_job_repository;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::ImageJob;

pub use sqlite_image_job_repository::SqliteImageJobRepository;

#[async_trait]
pub trait ImageJobRepository: Send + Sync {
    async fn insert(&self, job: &ImageJob) -> Result<(), RepositoryError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ImageJob>, RepositoryError>;

    async fn claim_next_available_job(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Option<ImageJob>, RepositoryError>;

    async fn save(&self, job: &ImageJob) -> Result<(), RepositoryError>;
}

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("invalid uuid in database: {0}")]
    InvalidUuid(String),

    #[error("invalid datetime in database field `{field}`: {value}")]
    InvalidDateTime { field: String, value: String },

    #[error("invalid image status in database: {0}")]
    InvalidStatus(String),
}
