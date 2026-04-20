# actix-web Remote API 実装プラン

## Context

空の Rust プロジェクトに actix-web で `/api/v1/` と `/api/v2/` を実装する。
両バージョンの違いは未知クエリパラメータの扱い: v1 は無視、v2 はエラー。
全レスポンスに `result` (bool) を含める: 成功時 `true`、失敗時 `false`。
エラーはすべて `{"result": false, "code": "...", "message": "..."}` の JSON で自動返却。

---

## ファイル構成

```
remoteapi-test/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── errors.rs
    └── routes/
        ├── mod.rs
        ├── v1/
        │   ├── mod.rs
        │   └── hello.rs
        └── v2/
            ├── mod.rs
            └── hello.rs
```

---

## Cargo.toml

```toml
[package]
name = "remoteapi-test"
version = "0.1.0"
edition = "2021"

[dependencies]
actix-web  = "4"
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror  = "2"
```

`form_urlencoded` は actix-web の推移依存として既に含まれるため追加不要。

---

## エラー設計 (`src/errors.rs`)

`ErrorCode` enum を独立して定義し、`code` フィールドの型として使用する。
serde の `rename_all = "SCREAMING_SNAKE_CASE"` で JSON には `"UNKNOWN_QUERY_PARAM"` 形式で出力される。

```rust
use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde::Serialize;
use thiserror::Error;

/// JSON の "code" フィールドに使われる enum。
/// serde により SCREAMING_SNAKE_CASE の文字列としてシリアライズされる。
#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    UnknownQueryParam,
    MissingParam,
    InvalidParam,
}

#[derive(Serialize)]
struct ErrorBody {
    result: bool,       // 常に false
    code: ErrorCode,
    message: String,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Unknown query parameter: {0}")]
    UnknownQueryParam(String),

    #[error("Missing required parameter: {0}")]
    MissingParam(String),

    #[error("Invalid parameter value for '{field}': {reason}")]
    InvalidParam { field: &'static str, reason: String },
}

impl ApiError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::UnknownQueryParam(_) => ErrorCode::UnknownQueryParam,
            Self::MissingParam(_)      => ErrorCode::MissingParam,
            Self::InvalidParam { .. }  => ErrorCode::InvalidParam,
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode { StatusCode::BAD_REQUEST }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(ErrorBody {
            result: false,
            code: self.code(),
            message: self.to_string(),
        })
    }
}
```

- `ErrorCode` は `ApiError` と 1:1 対応。新しいエラー追加時は両方に variant を追加する
- `thiserror` が `Display` を自動生成 → `message` フィールドへそのまま使用
- `ResponseError` の実装が JSON 返却を一元管理

---

## V1 ハンドラ (`src/routes/v1/hello.rs`)

`web::Query<T>` は内部で `serde_urlencoded` を使用しており、未知フィールドを自動で無視する。追加処理不要。

```rust
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
```

---

## V2 ハンドラ (`src/routes/v2/hello.rs`)

`web::Query` + `serde_urlencoded` は `deny_unknown_fields` を無視するため、二段階パースで対処:
1. 生クエリ文字列を `form_urlencoded::parse` でキー一覧取得
2. `KNOWN_KEYS` と比較して未知キーがあれば `ApiError::UnknownQueryParam` を返す
3. 通常の typed パースで処理

```rust
use actix_web::{web, HttpRequest, HttpResponse, Result};
use serde::Deserialize;
use std::collections::HashSet;
use crate::errors::ApiError;

#[derive(Deserialize)]
struct HelloParams {
    name: Option<String>,
}

const KNOWN_KEYS: &[&str] = &["name"];

pub async fn hello(req: HttpRequest, params: web::Query<HelloParams>) -> Result<HttpResponse, ApiError> {
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
```

---

## ルート登録 (`src/routes/`)

`v1/mod.rs` と `v2/mod.rs` でそれぞれ `configure` 関数を定義し、`main.rs` で呼び出す。

```rust
// main.rs
App::new()
    .configure(routes::v1::configure)
    .configure(routes::v2::configure)
```

---

## 実装順序

1. `Cargo.toml` 作成
2. `src/errors.rs` — 先にエラー型を確定
3. `src/routes/v1/` — 動作確認しやすいシンプルな側から
4. `src/routes/v2/` — 二段階パースを追加
5. `src/main.rs` — 両ルートを結合

---

## 動作確認

```bash
cargo run

# v1: 未知パラメータは無視される
curl "http://localhost:8080/api/v1/hello?name=Alice&foo=bar"
# → {"result":true,"message":"Hello, Alice!"}

# v2: 正常
curl "http://localhost:8080/api/v2/hello?name=Alice"
# → {"result":true,"message":"Hello, Alice!"}

# v2: 未知パラメータはエラー
curl "http://localhost:8080/api/v2/hello?name=Alice&foo=bar"
# → {"result":false,"code":"UNKNOWN_QUERY_PARAM","message":"Unknown query parameter: foo"}  (400)

# v2: パラメータなし (optional なので OK)
curl "http://localhost:8080/api/v2/hello"
# → {"result":true,"message":"Hello, world!"}
```
