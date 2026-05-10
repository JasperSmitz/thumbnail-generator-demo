use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
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

        Self { host, port }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}