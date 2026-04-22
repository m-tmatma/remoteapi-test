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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, App};

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

    #[actix_web::test]
    async fn returns_default_greeting_without_name() {
        let app = test::init_service(App::new().route("/hello", web::get().to(hello))).await;
        let req = test::TestRequest::get().uri("/hello").to_request();
        let body: serde_json::Value = test::read_body_json(test::call_service(&app, req).await).await;
        assert_eq!(body["message"], "Hello, world!");
    }

    #[actix_web::test]
    async fn ignores_unknown_params() {
        let app = test::init_service(App::new().route("/hello", web::get().to(hello))).await;
        let req = test::TestRequest::get().uri("/hello?name=Alice&foo=bar").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "Hello, Alice!");
    }
}
