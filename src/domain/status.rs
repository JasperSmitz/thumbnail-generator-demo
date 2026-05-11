use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageJobStatus {
    Pending,
    Processing,
    Done,
    Failed,
}

impl ImageJobStatus {
    pub fn is_terminal(&self) -> bool {
        match self {
            ImageJobStatus::Done => true,
            ImageJobStatus::Failed => false,
            ImageJobStatus::Pending => false,
            ImageJobStatus::Processing => false,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ImageJobStatus::Pending => "pending",
            ImageJobStatus::Processing => "processing",
            ImageJobStatus::Done => "done",
            ImageJobStatus::Failed => "failed",
        }
    }

    pub fn from_str(value: &str) -> Result<Self, ImageJobStatusParseError> {
        match value {
            "pending" => Ok(ImageJobStatus::Pending),
            "processing" => Ok(ImageJobStatus::Processing),
            "done" => Ok(ImageJobStatus::Done),
            "failed" => Ok(ImageJobStatus::Failed),
            other => Err(ImageJobStatusParseError {
                value: other.to_string(),
            }),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("invalid image job status: {value}")]
pub struct ImageJobStatusParseError {
    pub value: String,
}
