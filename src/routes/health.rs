use actix_web::{HttpResponse, web};

pub fn configure(config: &mut web::ServiceConfig) {
    config.route("/health", web::get().to(health));
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        service: "image-indexer-demo",
    })
}

#[derive(serde::Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}
