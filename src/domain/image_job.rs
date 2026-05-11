use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::status::ImageJobStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageJob {
    pub id: Uuid,
    pub original_filename: String,
    pub stored_path: String,
    pub thumbnail_path: Option<String>,
    pub status: ImageJobStatus,
    pub attempts: u32,
    pub max_attempts: u32,
    pub last_error: Option<String>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub indexed_at: Option<DateTime<Utc>>,
}

impl ImageJob {
    pub fn new(
        original_filename: String,
        stored_path: String,
        max_attempts: u32,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            original_filename,
            stored_path,
            thumbnail_path: None,
            status: ImageJobStatus::Pending,
            attempts: 0,
            max_attempts,
            last_error: None,
            next_retry_at: None,
            created_at: now,
            updated_at: now,
            indexed_at: None,
        }
    }

    pub fn mark_processing(&mut self, now: DateTime<Utc>) -> Result<(), ImageJobTransitionError> {
        match self.status {
            ImageJobStatus::Pending => {
                self.status = ImageJobStatus::Processing;
                self.updated_at = now;
                Ok(())
            }
            ImageJobStatus::Failed => {
                if self.can_retry(now) {
                    self.status = ImageJobStatus::Processing;
                    self.updated_at = now;
                    Ok(())
                } else {
                    Err(ImageJobTransitionError::RetryNotAllowed)
                }
            }
            ImageJobStatus::Processing => Err(ImageJobTransitionError::AlreadyProcessing),
            ImageJobStatus::Done => Err(ImageJobTransitionError::AlreadyDone),
        }
    }

    pub fn mark_done(
        &mut self,
        thumbnail_path: String,
        now: DateTime<Utc>,
    ) -> Result<(), ImageJobTransitionError> {
        match self.status {
            ImageJobStatus::Processing => {
                self.status = ImageJobStatus::Done;
                self.thumbnail_path = Some(thumbnail_path);
                self.last_error = None;
                self.next_retry_at = None;
                self.indexed_at = Some(now);
                self.updated_at = now;
                Ok(())
            }
            ImageJobStatus::Pending => Err(ImageJobTransitionError::NotProcessing),
            ImageJobStatus::Failed => Err(ImageJobTransitionError::NotProcessing),
            ImageJobStatus::Done => Err(ImageJobTransitionError::AlreadyDone),
        }
    }

    pub fn mark_failed(
        &mut self,
        error: String,
        next_retry_at: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> Result<(), ImageJobTransitionError> {
        match self.status {
            ImageJobStatus::Processing => {
                self.status = ImageJobStatus::Failed;
                self.attempts = self.attempts + 1;
                self.last_error = Some(error);
                self.updated_at = now;

                if self.attempts >= self.max_attempts {
                    self.next_retry_at = None;
                } else {
                    self.next_retry_at = next_retry_at;
                }

                Ok(())
            }
            ImageJobStatus::Pending => Err(ImageJobTransitionError::NotProcessing),
            ImageJobStatus::Failed => Err(ImageJobTransitionError::NotProcessing),
            ImageJobStatus::Done => Err(ImageJobTransitionError::AlreadyDone),
        }
    }

    pub fn can_retry(&self, now: DateTime<Utc>) -> bool {
        match self.status {
            ImageJobStatus::Failed => {}
            ImageJobStatus::Pending => return true,
            ImageJobStatus::Processing => return false,
            ImageJobStatus::Done => return false,
        }

        if self.attempts >= self.max_attempts {
            return false;
        }

        match self.next_retry_at {
            Some(next_retry_at) => next_retry_at <= now,
            None => true,
        }
    }

    pub fn is_available_for_processing(&self, now: DateTime<Utc>) -> bool {
        match self.status {
            ImageJobStatus::Pending => true,
            ImageJobStatus::Failed => self.can_retry(now),
            ImageJobStatus::Processing => false,
            ImageJobStatus::Done => false,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ImageJobTransitionError {
    #[error("image job is already processing")]
    AlreadyProcessing,

    #[error("image job is already done")]
    AlreadyDone,

    #[error("image job must be processing before it can be completed or failed")]
    NotProcessing,

    #[error("image job is not eligible for retry yet")]
    RetryNotAllowed,
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::domain::{ImageJob, ImageJobStatus};

    #[test]
    fn new_job_starts_as_pending() {
        let now = Utc::now();

        let job = ImageJob::new(
            "room.jpg".to_string(),
            "storage/originals/room.jpg".to_string(),
            3,
            now,
        );

        assert_eq!(job.status, ImageJobStatus::Pending);
        assert_eq!(job.attempts, 0);
        assert_eq!(job.max_attempts, 3);
        assert_eq!(job.thumbnail_path, None);
        assert_eq!(job.last_error, None);
        assert_eq!(job.next_retry_at, None);
        assert_eq!(job.indexed_at, None);
    }

    #[test]
    fn pending_job_can_transition_to_processing() {
        let now = Utc::now();

        let mut job = ImageJob::new(
            "room.jpg".to_string(),
            "storage/originals/room.jpg".to_string(),
            3,
            now,
        );

        let result = job.mark_processing(now);

        assert_eq!(result, Ok(()));
        assert_eq!(job.status, ImageJobStatus::Processing);
    }

    #[test]
    fn processing_job_can_transition_to_done() {
        let now = Utc::now();

        let mut job = ImageJob::new(
            "room.jpg".to_string(),
            "storage/originals/room.jpg".to_string(),
            3,
            now,
        );

        let processing_result = job.mark_processing(now);
        assert_eq!(processing_result, Ok(()));

        let done_result = job.mark_done("storage/thumbnails/room.jpg".to_string(), now);

        assert_eq!(done_result, Ok(()));
        assert_eq!(job.status, ImageJobStatus::Done);
        assert_eq!(
            job.thumbnail_path,
            Some("storage/thumbnails/room.jpg".to_string())
        );
        assert_eq!(job.last_error, None);
        assert_eq!(job.next_retry_at, None);
        assert_eq!(job.indexed_at, Some(now));
    }

    #[test]
    fn processing_job_can_transition_to_failed() {
        let now = Utc::now();
        let retry_at = now + Duration::seconds(10);

        let mut job = ImageJob::new(
            "room.jpg".to_string(),
            "storage/originals/room.jpg".to_string(),
            3,
            now,
        );

        let processing_result = job.mark_processing(now);
        assert_eq!(processing_result, Ok(()));

        let failed_result = job.mark_failed(
            "thumbnail generation failed".to_string(),
            Some(retry_at),
            now,
        );

        assert_eq!(failed_result, Ok(()));
        assert_eq!(job.status, ImageJobStatus::Failed);
        assert_eq!(job.attempts, 1);
        assert_eq!(
            job.last_error,
            Some("thumbnail generation failed".to_string())
        );
        assert_eq!(job.next_retry_at, Some(retry_at));
    }

    #[test]
    fn failed_job_can_retry_after_retry_time_has_passed() {
        let now = Utc::now();
        let retry_at = now - Duration::seconds(1);

        let mut job = ImageJob::new(
            "room.jpg".to_string(),
            "storage/originals/room.jpg".to_string(),
            3,
            now,
        );

        let processing_result = job.mark_processing(now);
        assert_eq!(processing_result, Ok(()));

        let failed_result = job.mark_failed("temporary failure".to_string(), Some(retry_at), now);
        assert_eq!(failed_result, Ok(()));

        assert_eq!(job.can_retry(now), true);

        let retry_result = job.mark_processing(now);

        assert_eq!(retry_result, Ok(()));
        assert_eq!(job.status, ImageJobStatus::Processing);
    }

    #[test]
    fn failed_job_cannot_retry_before_retry_time() {
        let now = Utc::now();
        let retry_at = now + Duration::seconds(60);

        let mut job = ImageJob::new(
            "room.jpg".to_string(),
            "storage/originals/room.jpg".to_string(),
            3,
            now,
        );

        let processing_result = job.mark_processing(now);
        assert_eq!(processing_result, Ok(()));

        let failed_result = job.mark_failed("temporary failure".to_string(), Some(retry_at), now);
        assert_eq!(failed_result, Ok(()));

        assert_eq!(job.can_retry(now), false);

        let retry_result = job.mark_processing(now);

        assert_eq!(retry_result.is_err(), true);
        assert_eq!(job.status, ImageJobStatus::Failed);
    }

    #[test]
    fn failed_job_cannot_retry_after_max_attempts() {
        let now = Utc::now();
        let retry_at = now - Duration::seconds(1);

        let mut job = ImageJob::new(
            "room.jpg".to_string(),
            "storage/originals/room.jpg".to_string(),
            1,
            now,
        );

        let processing_result = job.mark_processing(now);
        assert_eq!(processing_result, Ok(()));

        let failed_result = job.mark_failed("permanent failure".to_string(), Some(retry_at), now);
        assert_eq!(failed_result, Ok(()));

        assert_eq!(job.status, ImageJobStatus::Failed);
        assert_eq!(job.attempts, 1);
        assert_eq!(job.next_retry_at, None);
        assert_eq!(job.can_retry(now), false);
    }

    #[test]
    fn pending_job_cannot_transition_directly_to_done() {
        let now = Utc::now();

        let mut job = ImageJob::new(
            "room.jpg".to_string(),
            "storage/originals/room.jpg".to_string(),
            3,
            now,
        );

        let result = job.mark_done("storage/thumbnails/room.jpg".to_string(), now);

        assert_eq!(result.is_err(), true);
        assert_eq!(job.status, ImageJobStatus::Pending);
    }
}
