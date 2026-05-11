use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub storage_root: String,
    pub max_attempts: u32,
    pub max_upload_bytes: usize,
}

impl Config {
    pub fn from_env() -> Self {
        let host = match env::var("HOST") {
            Ok(value) => value,
            Err(_) => "127.0.0.1".to_string(),
        };

        let port = match env::var("PORT") {
            Ok(value) => match value.parse::<u16>() {
                Ok(parsed) => parsed,
                Err(_) => 8080,
            },
            Err(_) => 8080,
        };

        let database_url = match env::var("DATABASE_URL") {
            Ok(value) => value,
            Err(_) => "sqlite://image-indexer-demo.db".to_string(),
        };

        let storage_root = match env::var("STORAGE_ROOT") {
            Ok(value) => value,
            Err(_) => "storage".to_string(),
        };

        let max_attempts = match env::var("MAX_ATTEMPTS") {
            Ok(value) => match value.parse::<u32>() {
                Ok(parsed) => parsed,
                Err(_) => 3,
            },
            Err(_) => 3,
        };

        let max_upload_bytes = match env::var("MAX_UPLOAD_BYTES") {
            Ok(value) => match value.parse::<usize>() {
                Ok(parsed) => parsed,
                Err(_) => 10 * 1024 * 1024,
            },
            Err(_) => 10 * 1024 * 1024,
        };

        Self {
            host,
            port,
            database_url,
            storage_root,
            max_attempts,
            max_upload_bytes,
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
