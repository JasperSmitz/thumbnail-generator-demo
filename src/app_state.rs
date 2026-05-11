use std::sync::Arc;

use crate::config::Config;
use crate::repository::ImageJobRepository;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub image_jobs: Arc<dyn ImageJobRepository>,
}

impl AppState {
    pub fn new(config: Config, image_jobs: Arc<dyn ImageJobRepository>) -> Self {
        Self { config, image_jobs }
    }
}
