use std::path::Path;

use async_trait::async_trait;
use thiserror::Error;

#[async_trait]
pub trait ImageProcessor: Send + Sync {
    async fn process(&self, input_path: &Path, output_path: &Path) -> Result<(), ProcessingError>;
}

#[derive(Debug, Error)]
pub enum ProcessingError {
    #[error("failed to read image: {0}")]
    ReadImage(String),

    #[error("failed to write image: {0}")]
    WriteImage(String),

    #[error("processing task failed: {0}")]
    TaskJoin(String),
}
