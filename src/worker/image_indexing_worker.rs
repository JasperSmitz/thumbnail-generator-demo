use std::path::Path;
use std::sync::Arc;

use chrono::{Duration as ChronoDuration, Utc};
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};

use crate::processing::ImageProcessor;
use crate::repository::ImageJobRepository;
use crate::storage::{build_thumbnail_path, path_to_string};

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub storage_root: String,
    pub idle_sleep_ms: u64,
    pub failure_retry_delay_seconds: i64,
}

impl WorkerConfig {
    pub fn new(storage_root: String) -> Self {
        Self {
            storage_root,
            idle_sleep_ms: 500,
            failure_retry_delay_seconds: 30,
        }
    }
}

pub struct ImageIndexingWorker {
    repository: Arc<dyn ImageJobRepository>,
    processor: Arc<dyn ImageProcessor>,
    config: WorkerConfig,
}

impl ImageIndexingWorker {
    pub fn new(
        repository: Arc<dyn ImageJobRepository>,
        processor: Arc<dyn ImageProcessor>,
        config: WorkerConfig,
    ) -> Self {
        Self {
            repository,
            processor,
            config,
        }
    }

    pub async fn run(self) {
        info!("Image indexing worker started");

        loop {
            let processed_job = self.process_next_job().await;

            match processed_job {
                true => {}
                false => {
                    sleep(Duration::from_millis(self.config.idle_sleep_ms)).await;
                }
            }
        }
    }

    async fn process_next_job(&self) -> bool {
        let now = Utc::now();

        let claimed_job_result = self.repository.claim_next_available_job(now).await;

        let mut job = match claimed_job_result {
            Ok(Some(value)) => value,
            Ok(None) => return false,
            Err(error) => {
                error!("Worker failed to claim next image job: {}", error);
                return false;
            }
        };

        info!("Worker claimed image job {}", job.id);

        let thumbnail_path = build_thumbnail_path(&self.config.storage_root, job.id);

        let processing_result = self
            .processor
            .process(Path::new(&job.stored_path), &thumbnail_path)
            .await;

        match processing_result {
            Ok(_) => {
                let thumbnail_path_string = match path_to_string(&thumbnail_path) {
                    Ok(value) => value,
                    Err(error) => {
                        self.mark_job_failed(
                            &mut job,
                            format!("failed to convert thumbnail path to string: {error}"),
                        )
                        .await;

                        return true;
                    }
                };

                let done_result = job.mark_done(thumbnail_path_string, Utc::now());

                match done_result {
                    Ok(_) => {}
                    Err(error) => {
                        error!(
                            "Worker failed to transition job {} to done: {}",
                            job.id, error
                        );
                        return true;
                    }
                }

                match self.repository.save(&job).await {
                    Ok(_) => {
                        info!("Worker completed image job {}", job.id);
                    }
                    Err(error) => {
                        error!("Worker failed to save completed job {}: {}", job.id, error);
                    }
                }
            }
            Err(error) => {
                warn!("Worker failed to process image job {}: {}", job.id, error);

                self.mark_job_failed(&mut job, error.to_string()).await;
            }
        }

        true
    }

    async fn mark_job_failed(&self, job: &mut crate::domain::ImageJob, message: String) {
        let now = Utc::now();

        let next_retry_at = match job.attempts + 1 >= job.max_attempts {
            true => None,
            false => Some(now + ChronoDuration::seconds(self.config.failure_retry_delay_seconds)),
        };

        let failed_result = job.mark_failed(message, next_retry_at, now);

        match failed_result {
            Ok(_) => {}
            Err(error) => {
                error!(
                    "Worker failed to transition job {} to failed: {}",
                    job.id, error
                );
                return;
            }
        }

        match self.repository.save(job).await {
            Ok(_) => {
                warn!(
                    "Worker marked image job {} as failed; attempts={}/{}",
                    job.id, job.attempts, job.max_attempts
                );
            }
            Err(error) => {
                error!("Worker failed to save failed job {}: {}", job.id, error);
            }
        }
    }
}
