pub mod hello;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v2")
            .route("/hello", web::get().to(hello::hello)),
    );
}
