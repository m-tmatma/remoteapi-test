use actix_web::{web, HttpRequest, HttpResponse, Result};
use serde::Deserialize;
use std::collections::HashSet;
use crate::errors::ApiError;

#[derive(Deserialize)]
pub struct HelloParams {
    name: Option<String>,
}

const KNOWN_KEYS: &[&str] = &["name"];

pub async fn hello(
    req: HttpRequest,
    params: web::Query<HelloParams>,
) -> Result<HttpResponse, ApiError> {
    let known: HashSet<&str> = KNOWN_KEYS.iter().copied().collect();
    if let Some(qs) = req.uri().query() {
        for (key, _) in form_urlencoded::parse(qs.as_bytes()) {
            if !known.contains(key.as_ref()) {
                return Err(ApiError::UnknownQueryParam(key.into_owned()));
            }
        }
    }
    let name = params.name.as_deref().unwrap_or("world");
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": true,
        "message": format!("Hello, {}!", name)
    })))
}
