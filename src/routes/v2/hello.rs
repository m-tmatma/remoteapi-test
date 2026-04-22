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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, App};

    #[actix_web::test]
    async fn returns_greeting_with_known_param() {
        let app = test::init_service(App::new().route("/hello", web::get().to(hello))).await;
        let req = test::TestRequest::get().uri("/hello?name=Alice").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["result"],  true);
        assert_eq!(body["message"], "Hello, Alice!");
    }

    #[actix_web::test]
    async fn returns_default_greeting_without_params() {
        let app = test::init_service(App::new().route("/hello", web::get().to(hello))).await;
        let req = test::TestRequest::get().uri("/hello").to_request();
        let body: serde_json::Value = test::read_body_json(test::call_service(&app, req).await).await;
        assert_eq!(body["message"], "Hello, world!");
    }

    #[actix_web::test]
    async fn rejects_unknown_param() {
        let app = test::init_service(App::new().route("/hello", web::get().to(hello))).await;
        let req = test::TestRequest::get().uri("/hello?name=Alice&foo=bar").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["result"], false);
        assert_eq!(body["code"],   1001);
    }

    #[actix_web::test]
    async fn rejects_only_unknown_param() {
        let app = test::init_service(App::new().route("/hello", web::get().to(hello))).await;
        let req = test::TestRequest::get().uri("/hello?bar=baz").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["code"], 1001);
    }
}
