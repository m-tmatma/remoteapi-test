mod auth;
mod errors;
mod routes;

use actix_web::{App, HttpServer};

const PORT: u16 = 8080;
use auth::{CredentialsFile, DigestAuth, DigestAuthConfig};
use std::sync::Arc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let creds: CredentialsFile = {
        let data = std::fs::read_to_string("credentials.json")
            .expect("credentials.json not found");
        serde_json::from_str(&data).expect("invalid credentials.json")
    };

    let auth_config = Arc::new(DigestAuthConfig::new(creds.realm, creds.users));

    HttpServer::new(move || {
        App::new()
            .wrap(DigestAuth(auth_config.clone()))
            .configure(routes::v1::configure)
            .configure(routes::v2::configure)
    })
    .bind(("0.0.0.0", PORT))?
    .run()
    .await
}
