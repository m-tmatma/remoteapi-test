pub mod hello;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/hello", web::get().to(hello::hello))
            .route("/show_hello", web::get().to(hello::show_hello)),
    );
}
