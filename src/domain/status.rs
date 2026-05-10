use serde::{Deserialize, Serialize};

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
}