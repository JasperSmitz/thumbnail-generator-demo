mod app_state;
mod config;
mod domain;
mod error;
mod routes;

use actix_web::{App, HttpServer, web};
use app_state::AppState;
use config::Config;
use dotenvy::dotenv;
use tracing::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    match dotenv() {
        Ok(_) => {}
        Err(_) => {}
    }

    init_logging();

    let config = Config::from_env();
    let bind_address = config.bind_address();

    let state = web::Data::new(AppState::new(config));

    info!("Starting image-indexer-demo on {}", bind_address);

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .configure(routes::configure_routes)
    })
    .bind(bind_address)?
    .run()
    .await
}

fn init_logging() {
    let rust_log = match std::env::var("RUST_LOG") {
        Ok(value) => value,
        Err(_) => "image_indexer_demo=info,actix_web=info".to_string(),
    };

    tracing_subscriber::fmt().with_env_filter(rust_log).init();
}
