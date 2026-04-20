use actix_web::{web, HttpResponse, Result};
use serde::Deserialize;
use crate::errors::ApiError;

#[derive(Deserialize)]
pub struct HelloParams {
    name: Option<String>,
}

pub async fn hello(params: web::Query<HelloParams>) -> Result<HttpResponse, ApiError> {
    let name = params.name.as_deref().unwrap_or("world");
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": true,
        "message": format!("Hello, {}!", name)
    })))
}
