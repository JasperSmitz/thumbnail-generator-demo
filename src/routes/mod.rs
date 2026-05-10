pub mod health;

use actix_web::web;

pub fn configure_routes(config: &mut web::ServiceConfig) {
    config.configure(health::configure);
}
