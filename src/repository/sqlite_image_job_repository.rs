use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions, SqliteRow};
use uuid::Uuid;

use crate::domain::{ImageJob, ImageJobStatus};
use crate::repository::{ImageJobRepository, RepositoryError};

#[derive(Debug, Clone)]
pub struct SqliteImageJobRepository {
    pool: SqlitePool,
}

impl SqliteImageJobRepository {
    pub async fn connect(database_url: &str) -> Result<Self, RepositoryError> {
        let options = match SqliteConnectOptions::from_str(database_url) {
            Ok(value) => value.create_if_missing(true),
            Err(error) => {
                return Err(RepositoryError::Database(sqlx::Error::Configuration(
                    Box::new(error),
                )));
            }
        };

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        let repository = Self { pool };

        repository.run_migrations().await?;

        Ok(repository)
    }

    async fn run_migrations(&self) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS image_jobs (
                id TEXT PRIMARY KEY NOT NULL,
                original_filename TEXT NOT NULL,
                stored_path TEXT NOT NULL,
                thumbnail_path TEXT NULL,
                status TEXT NOT NULL,
                attempts INTEGER NOT NULL,
                max_attempts INTEGER NOT NULL,
                last_error TEXT NULL,
                next_retry_at TEXT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                indexed_at TEXT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_image_jobs_status_created_at
            ON image_jobs(status, created_at);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_image_jobs_retry
            ON image_jobs(status, next_retry_at, attempts, max_attempts);
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    fn map_row(row: SqliteRow) -> Result<ImageJob, RepositoryError> {
        let id_value: String = row.try_get("id")?;
        let id = match Uuid::parse_str(&id_value) {
            Ok(parsed) => parsed,
            Err(_) => return Err(RepositoryError::InvalidUuid(id_value)),
        };

        let status_value: String = row.try_get("status")?;
        let status = match ImageJobStatus::from_str(&status_value) {
            Ok(parsed) => parsed,
            Err(_) => return Err(RepositoryError::InvalidStatus(status_value)),
        };

        let attempts_value: i64 = row.try_get("attempts")?;
        let attempts = match u32::try_from(attempts_value) {
            Ok(parsed) => parsed,
            Err(_) => {
                return Err(RepositoryError::Database(sqlx::Error::ColumnDecode {
                    index: "attempts".to_string(),
                    source: "attempts cannot be converted to u32".into(),
                }));
            }
        };

        let max_attempts_value: i64 = row.try_get("max_attempts")?;
        let max_attempts = match u32::try_from(max_attempts_value) {
            Ok(parsed) => parsed,
            Err(_) => {
                return Err(RepositoryError::Database(sqlx::Error::ColumnDecode {
                    index: "max_attempts".to_string(),
                    source: "max_attempts cannot be converted to u32".into(),
                }));
            }
        };

        let created_at_value: String = row.try_get("created_at")?;
        let created_at = parse_datetime("created_at", &created_at_value)?;

        let updated_at_value: String = row.try_get("updated_at")?;
        let updated_at = parse_datetime("updated_at", &updated_at_value)?;

        let next_retry_at_value: Option<String> = row.try_get("next_retry_at")?;
        let next_retry_at = match next_retry_at_value {
            Some(value) => Some(parse_datetime("next_retry_at", &value)?),
            None => None,
        };

        let indexed_at_value: Option<String> = row.try_get("indexed_at")?;
        let indexed_at = match indexed_at_value {
            Some(value) => Some(parse_datetime("indexed_at", &value)?),
            None => None,
        };

        Ok(ImageJob {
            id,
            original_filename: row.try_get("original_filename")?,
            stored_path: row.try_get("stored_path")?,
            thumbnail_path: row.try_get("thumbnail_path")?,
            status,
            attempts,
            max_attempts,
            last_error: row.try_get("last_error")?,
            next_retry_at,
            created_at,
            updated_at,
            indexed_at,
        })
    }
}

#[async_trait::async_trait]
impl ImageJobRepository for SqliteImageJobRepository {
    async fn insert(&self, job: &ImageJob) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO image_jobs (
                id,
                original_filename,
                stored_path,
                thumbnail_path,
                status,
                attempts,
                max_attempts,
                last_error,
                next_retry_at,
                created_at,
                updated_at,
                indexed_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
            "#,
        )
        .bind(job.id.to_string())
        .bind(&job.original_filename)
        .bind(&job.stored_path)
        .bind(&job.thumbnail_path)
        .bind(job.status.as_str())
        .bind(i64::from(job.attempts))
        .bind(i64::from(job.max_attempts))
        .bind(&job.last_error)
        .bind(optional_datetime_to_string(job.next_retry_at))
        .bind(datetime_to_string(job.created_at))
        .bind(datetime_to_string(job.updated_at))
        .bind(optional_datetime_to_string(job.indexed_at))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ImageJob>, RepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT
                id,
                original_filename,
                stored_path,
                thumbnail_path,
                status,
                attempts,
                max_attempts,
                last_error,
                next_retry_at,
                created_at,
                updated_at,
                indexed_at
            FROM image_jobs
            WHERE id = ?;
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(found_row) => Ok(Some(Self::map_row(found_row)?)),
            None => Ok(None),
        }
    }

    async fn claim_next_available_job(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Option<ImageJob>, RepositoryError> {
        let now_string = datetime_to_string(now);

        let row = sqlx::query(
            r#"
            UPDATE image_jobs
            SET
                status = 'processing',
                updated_at = ?
            WHERE id = (
                SELECT id
                FROM image_jobs
                WHERE
                    status = 'pending'
                    OR (
                        status = 'failed'
                        AND attempts < max_attempts
                        AND (
                            next_retry_at IS NULL
                            OR next_retry_at <= ?
                        )
                    )
                ORDER BY
                    CASE status
                        WHEN 'pending' THEN 0
                        ELSE 1
                    END,
                    COALESCE(next_retry_at, created_at),
                    created_at
                LIMIT 1
            )
            RETURNING
                id,
                original_filename,
                stored_path,
                thumbnail_path,
                status,
                attempts,
                max_attempts,
                last_error,
                next_retry_at,
                created_at,
                updated_at,
                indexed_at;
            "#,
        )
        .bind(&now_string)
        .bind(&now_string)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(found_row) => Ok(Some(Self::map_row(found_row)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, job: &ImageJob) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            UPDATE image_jobs
            SET
                original_filename = ?,
                stored_path = ?,
                thumbnail_path = ?,
                status = ?,
                attempts = ?,
                max_attempts = ?,
                last_error = ?,
                next_retry_at = ?,
                created_at = ?,
                updated_at = ?,
                indexed_at = ?
            WHERE id = ?;
            "#,
        )
        .bind(&job.original_filename)
        .bind(&job.stored_path)
        .bind(&job.thumbnail_path)
        .bind(job.status.as_str())
        .bind(i64::from(job.attempts))
        .bind(i64::from(job.max_attempts))
        .bind(&job.last_error)
        .bind(optional_datetime_to_string(job.next_retry_at))
        .bind(datetime_to_string(job.created_at))
        .bind(datetime_to_string(job.updated_at))
        .bind(optional_datetime_to_string(job.indexed_at))
        .bind(job.id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

fn datetime_to_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

fn optional_datetime_to_string(value: Option<DateTime<Utc>>) -> Option<String> {
    match value {
        Some(datetime) => Some(datetime_to_string(datetime)),
        None => None,
    }
}

fn parse_datetime(field: &str, value: &str) -> Result<DateTime<Utc>, RepositoryError> {
    match DateTime::parse_from_rfc3339(value) {
        Ok(parsed) => Ok(parsed.with_timezone(&Utc)),
        Err(_) => Err(RepositoryError::InvalidDateTime {
            field: field.to_string(),
            value: value.to_string(),
        }),
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::domain::{ImageJob, ImageJobStatus};
    use crate::repository::{ImageJobRepository, RepositoryError, SqliteImageJobRepository};

    async fn create_repository() -> Result<SqliteImageJobRepository, RepositoryError> {
        SqliteImageJobRepository::connect("sqlite::memory:").await
    }

    #[tokio::test]
    async fn inserted_job_can_be_found_by_id() -> Result<(), RepositoryError> {
        let repository = create_repository().await?;
        let now = Utc::now();

        let job = ImageJob::new(
            "image.jpg".to_string(),
            "storage/originals/image.jpg".to_string(),
            3,
            now,
        );

        repository.insert(&job).await?;

        let found = repository.find_by_id(job.id).await?;

        match found {
            Some(found_job) => {
                assert_eq!(found_job.id, job.id);
                assert_eq!(found_job.original_filename, "image.jpg");
                assert_eq!(found_job.status, ImageJobStatus::Pending);
                assert_eq!(found_job.attempts, 0);
            }
            None => {
                panic!("expected inserted job to be found");
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn missing_job_returns_none() -> Result<(), RepositoryError> {
        let repository = create_repository().await?;
        let id = uuid::Uuid::new_v4();

        let found = repository.find_by_id(id).await?;

        assert_eq!(found.is_none(), true);

        Ok(())
    }

    #[tokio::test]
    async fn claim_next_available_job_marks_pending_job_as_processing()
    -> Result<(), RepositoryError> {
        let repository = create_repository().await?;
        let now = Utc::now();

        let job = ImageJob::new(
            "image.jpg".to_string(),
            "storage/originals/image.jpg".to_string(),
            3,
            now,
        );

        repository.insert(&job).await?;

        let claimed = repository.claim_next_available_job(now).await?;

        match claimed {
            Some(claimed_job) => {
                assert_eq!(claimed_job.id, job.id);
                assert_eq!(claimed_job.status, ImageJobStatus::Processing);
            }
            None => {
                panic!("expected a pending job to be claimed");
            }
        }

        let persisted = repository.find_by_id(job.id).await?;

        match persisted {
            Some(persisted_job) => {
                assert_eq!(persisted_job.status, ImageJobStatus::Processing);
            }
            None => {
                panic!("expected claimed job to still exist");
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn claim_next_available_job_skips_failed_job_before_retry_time()
    -> Result<(), RepositoryError> {
        let repository = create_repository().await?;
        let now = Utc::now();
        let retry_at = now + Duration::minutes(5);

        let mut job = ImageJob::new(
            "image.jpg".to_string(),
            "storage/originals/image.jpg".to_string(),
            3,
            now,
        );

        let processing_result = job.mark_processing(now);
        assert_eq!(processing_result, Ok(()));

        let failed_result = job.mark_failed("temporary failure".to_string(), Some(retry_at), now);
        assert_eq!(failed_result, Ok(()));

        repository.insert(&job).await?;

        let claimed = repository.claim_next_available_job(now).await?;

        assert_eq!(claimed.is_none(), true);

        Ok(())
    }

    #[tokio::test]
    async fn claim_next_available_job_claims_failed_job_after_retry_time()
    -> Result<(), RepositoryError> {
        let repository = create_repository().await?;
        let now = Utc::now();
        let retry_at = now - Duration::seconds(1);

        let mut job = ImageJob::new(
            "image.jpg".to_string(),
            "storage/originals/image.jpg".to_string(),
            3,
            now,
        );

        let processing_result = job.mark_processing(now);
        assert_eq!(processing_result, Ok(()));

        let failed_result = job.mark_failed("temporary failure".to_string(), Some(retry_at), now);
        assert_eq!(failed_result, Ok(()));

        repository.insert(&job).await?;

        let claimed = repository.claim_next_available_job(now).await?;

        match claimed {
            Some(claimed_job) => {
                assert_eq!(claimed_job.id, job.id);
                assert_eq!(claimed_job.status, ImageJobStatus::Processing);
                assert_eq!(claimed_job.attempts, 1);
            }
            None => {
                panic!("expected retryable failed job to be claimed");
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn save_persists_status_changes() -> Result<(), RepositoryError> {
        let repository = create_repository().await?;
        let now = Utc::now();

        let mut job = ImageJob::new(
            "image.jpg".to_string(),
            "storage/originals/image.jpg".to_string(),
            3,
            now,
        );

        repository.insert(&job).await?;

        let processing_result = job.mark_processing(now);
        assert_eq!(processing_result, Ok(()));

        let done_result = job.mark_done("storage/thumbnails/image.jpg".to_string(), now);
        assert_eq!(done_result, Ok(()));

        repository.save(&job).await?;

        let found = repository.find_by_id(job.id).await?;

        match found {
            Some(found_job) => {
                assert_eq!(found_job.status, ImageJobStatus::Done);
                assert_eq!(
                    found_job.thumbnail_path,
                    Some("storage/thumbnails/image.jpg".to_string())
                );
                assert_eq!(found_job.indexed_at, Some(now));
            }
            None => {
                panic!("expected saved job to be found");
            }
        }

        Ok(())
    }
}
