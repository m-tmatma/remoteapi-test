# actix-web Remote API 実装プラン

## Context

空の Rust プロジェクトに actix-web で `/api/v1/` と `/api/v2/` を実装する。
両バージョンの違いは未知クエリパラメータの扱い: v1 は無視、v2 はエラー。
全レスポンスに `result` (bool) を含める: 成功時 `true`、失敗時 `false`。
エラーはすべて `{"result": false, "code": 2, "message": "..."}` の JSON で自動返却。`code` は数値 (u32)。
全エンドポイントに Digest 認証を適用。認証情報は `credentials.json` から読み込む。

---

## ファイル構成

```
remoteapi-test/
├── Cargo.toml
├── credentials.json
├── test.sh
└── src/
    ├── main.rs
    ├── auth.rs
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
actix-web       = "4"
serde           = { version = "1", features = ["derive"] }
serde_json      = "1"
thiserror       = "2"
form_urlencoded = "1"
md5             = "0.7"
rand            = "0.9"
```

---

## エラーコード (`src/errors.rs`)

`ErrorCode` は `#[repr(u32)]` で数値 enum として定義し、`Serialize` を手動実装することで JSON に数値で出力される。

```rust
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum ErrorCode {
    Unauthorized      = 1,
    UnknownQueryParam = 2,
    MissingParam      = 3,
    InvalidParam      = 4,
}

impl Serialize for ErrorCode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u32(*self as u32)
    }
}
```

`ApiError` に `ResponseError` を実装し、エラー返却を一元管理する。

---

## Digest 認証 (`src/auth.rs`)

`credentials.json` のフォーマット:
```json
{
  "realm": "Remote API",
  "users": [
    { "username": "admin", "password": "password" }
  ]
}
```

- 起動時にパスワードを `MD5(username:realm:password)` (HA1) にハッシュ化してメモリに保持
- 未認証リクエストには `401 + WWW-Authenticate: Digest ...` を返す
- nonce はリクエスト毎に発行し、使用後に破棄（再利用不可）
- actix-web の `Transform` トレイトでミドルウェアとして実装

---

## エンドポイント一覧

| パス | パラメータ | 備考 |
|------|-----------|------|
| `/api/v1/hello` | `name` (optional) | 未知パラメータ無視 |
| `/api/v2/hello` | `name` (optional) | 未知パラメータ拒否 |
| `/api/v1/show_hello` | `greeting` (必須), `name` (optional) | 未知パラメータ無視 |
| `/api/v2/show_hello` | `greeting` (必須), `name` (optional) | 未知パラメータ拒否 |

---

## 動作確認

```bash
cargo build && cargo run

# 認証なし → 401
curl "http://localhost:8080/api/v1/hello"
# → {"result":false,"code":1,"message":"Authentication required"}

# v1: 未知パラメータは無視
curl --digest -u admin:password "http://localhost:8080/api/v1/hello?name=Alice&foo=bar"
# → {"result":true,"message":"Hello, Alice!"}

# v2: 未知パラメータはエラー
curl --digest -u admin:password "http://localhost:8080/api/v2/hello?name=Alice&foo=bar"
# → {"result":false,"code":2,"message":"Unknown query parameter: foo"}

# show_hello: greeting 必須
curl --digest -u admin:password "http://localhost:8080/api/v1/show_hello?greeting=Hi&name=Alice"
# → {"result":true,"message":"Hi, Alice!"}

# greeting 省略 → 400
curl --digest -u admin:password "http://localhost:8080/api/v1/show_hello?name=Alice"
# → {"result":false,"code":3,"message":"Missing required parameter: greeting"}
```

## テスト

```bash
cargo test        # Rust 単体テスト (20 件)
bash test.sh      # curl 統合テスト (21 件)
```
