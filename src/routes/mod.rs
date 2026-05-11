pub mod health;
pub mod images;

use actix_web::web;

pub fn configure_routes(config: &mut web::ServiceConfig) {
    config
        .configure(health::configure)
        .configure(images::configure);
}
