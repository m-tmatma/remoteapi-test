use actix_web::{web, HttpRequest, HttpResponse, Result};
use serde::Deserialize;
use std::collections::HashSet;
use crate::errors::{ApiError, ErrorCode};

#[derive(Deserialize)]
pub struct HelloParams {
    name: Option<String>,
}

const HELLO_KNOWN_KEYS: &[&str] = &["name"];

pub async fn hello(
    req: HttpRequest,
    params: web::Query<HelloParams>,
) -> Result<HttpResponse, ApiError> {
    let known: HashSet<&str> = HELLO_KNOWN_KEYS.iter().copied().collect();
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

#[derive(Deserialize)]
pub struct ShowHelloParams {
    greeting: String,
    name: Option<String>,
}

const SHOW_HELLO_KNOWN_KEYS: &[&str] = &["greeting", "name"];

pub async fn show_hello(
    req: HttpRequest,
    params: web::Query<ShowHelloParams>,
) -> Result<HttpResponse, ApiError> {
    let known: HashSet<&str> = SHOW_HELLO_KNOWN_KEYS.iter().copied().collect();
    if let Some(qs) = req.uri().query() {
        for (key, _) in form_urlencoded::parse(qs.as_bytes()) {
            if !known.contains(key.as_ref()) {
                return Err(ApiError::UnknownQueryParam(key.into_owned()));
            }
        }
    }
    let greeting = params.greeting.as_str();
    let name = params.name.as_deref().unwrap_or("world");
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": true,
        "message": format!("{}, {}!", greeting, name)
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, App};

    // name を指定したとき "Hello, {name}!" が返ることを確認する
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

    // name を省略したとき "Hello, world!" が返ることを確認する
    #[actix_web::test]
    async fn returns_default_name_without_name_param() {
        let app = test::init_service(App::new().route("/hello", web::get().to(hello))).await;
        let req = test::TestRequest::get().uri("/hello").to_request();
        let body: serde_json::Value = test::read_body_json(test::call_service(&app, req).await).await;
        assert_eq!(body["message"], "Hello, world!");
    }

    // v2 では未知のパラメータを含むリクエストが 400 になり code=2 が返ることを確認する
    #[actix_web::test]
    async fn rejects_unknown_param() {
        let app = test::init_service(App::new().route("/hello", web::get().to(hello))).await;
        let req = test::TestRequest::get().uri("/hello?name=Alice&foo=bar").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["result"], false);
        assert_eq!(body["code"],   2);
    }

    // 既知パラメータなしで未知パラメータだけ渡しても 400 になることを確認する
    #[actix_web::test]
    async fn rejects_only_unknown_param() {
        let app = test::init_service(App::new().route("/hello", web::get().to(hello))).await;
        let req = test::TestRequest::get().uri("/hello?bar=baz").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["code"], ErrorCode::UnknownQueryParam as u32);
    }

    // greeting を指定したとき "{greeting}, {name}!" が返ることを確認する
    #[actix_web::test]
    async fn show_hello_uses_custom_greeting() {
        let app = test::init_service(App::new().route("/show_hello", web::get().to(show_hello))).await;
        let req = test::TestRequest::get().uri("/show_hello?greeting=Hi&name=Alice").to_request();
        let body: serde_json::Value = test::read_body_json(test::call_service(&app, req).await).await;
        assert_eq!(body["message"], "Hi, Alice!");
    }

    // greeting を省略したとき 400 が返ることを確認する
    #[actix_web::test]
    async fn show_hello_missing_greeting_returns_400() {
        let app = test::init_service(App::new().route("/show_hello", web::get().to(show_hello))).await;
        let req = test::TestRequest::get().uri("/show_hello?name=Alice").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // show_hello でも未知パラメータを含むリクエストが 400 になり code=2 が返ることを確認する
    #[actix_web::test]
    async fn show_hello_rejects_unknown_param() {
        let app = test::init_service(App::new().route("/show_hello", web::get().to(show_hello))).await;
        let req = test::TestRequest::get().uri("/show_hello?greeting=Hi&foo=bar").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["code"], ErrorCode::UnknownQueryParam as u32);
    }
}
