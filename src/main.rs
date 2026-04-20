mod errors;
mod routes;

use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .configure(routes::v1::configure)
            .configure(routes::v2::configure)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
