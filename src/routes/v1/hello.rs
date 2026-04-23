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

#[derive(Deserialize)]
pub struct ShowHelloParams {
    greeting: Option<String>,
    name: Option<String>,
}

pub async fn show_hello(params: web::Query<ShowHelloParams>) -> Result<HttpResponse, ApiError> {
    let greeting = params.greeting.as_deref()
        .ok_or_else(|| ApiError::MissingParam("greeting".to_string()))?;
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
    use crate::errors::ErrorCode;

    // name を指定したとき "Hello, {name}!" が返ることを確認する
    #[actix_web::test]
    async fn returns_greeting_with_name() {
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
    async fn returns_default_greeting_without_name() {
        let app = test::init_service(App::new().route("/hello", web::get().to(hello))).await;
        let req = test::TestRequest::get().uri("/hello").to_request();
        let body: serde_json::Value = test::read_body_json(test::call_service(&app, req).await).await;
        assert_eq!(body["message"], "Hello, world!");
    }

    // v1 では未知のパラメータを無視して正常応答することを確認する
    #[actix_web::test]
    async fn ignores_unknown_params() {
        let app = test::init_service(App::new().route("/hello", web::get().to(hello))).await;
        let req = test::TestRequest::get().uri("/hello?name=Alice&foo=bar").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Hello, Alice!");
    }

    // greeting を指定したとき "{greeting}, {name}!" が返ることを確認する
    #[actix_web::test]
    async fn show_hello_uses_custom_greeting() {
        let app = test::init_service(App::new().route("/show_hello", web::get().to(show_hello))).await;
        let req = test::TestRequest::get().uri("/show_hello?greeting=Hi&name=Alice").to_request();
        let body: serde_json::Value = test::read_body_json(test::call_service(&app, req).await).await;
        assert_eq!(body["message"], "Hi, Alice!");
    }

    // greeting を省略したとき 400 になり code=3, message が返ることを確認する
    #[actix_web::test]
    async fn show_hello_missing_greeting_returns_400() {
        let app = test::init_service(App::new().route("/show_hello", web::get().to(show_hello))).await;
        let req = test::TestRequest::get().uri("/show_hello?name=Alice").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["result"],  false);
        assert_eq!(body["code"],    ErrorCode::MissingParam as u32);
        assert_eq!(body["message"], "Missing required parameter: greeting");
    }
}
