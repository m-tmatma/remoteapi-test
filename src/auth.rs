use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{self, HeaderValue},
    Error, HttpResponse,
};
use crate::errors::ErrorCode;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    future::{ready, Future, Ready},
    pin::Pin,
    sync::{Arc, Mutex},
};

// ---------------------------------------------------------------------------
// Credentials file schema
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CredentialsFile {
    pub realm: String,
    pub users: Vec<UserEntry>,
}

#[derive(Deserialize)]
pub struct UserEntry {
    pub username: String,
    pub password: String,
}

// ---------------------------------------------------------------------------
// Shared auth state
// ---------------------------------------------------------------------------

pub struct DigestAuthConfig {
    pub realm: String,
    /// username -> HA1 = MD5(username:realm:password)
    users: HashMap<String, String>,
    nonces: Arc<Mutex<HashSet<String>>>,
}

impl DigestAuthConfig {
    pub fn new(realm: String, users: Vec<UserEntry>) -> Self {
        let ha1_map = users
            .into_iter()
            .map(|u| {
                let ha1 = format!(
                    "{:x}",
                    md5::compute(format!("{}:{}:{}", u.username, realm, u.password))
                );
                (u.username, ha1)
            })
            .collect();

        Self {
            realm,
            users: ha1_map,
            nonces: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    fn issue_nonce(&self) -> String {
        let nonce = format!("{:x}", md5::compute(format!("{:?}", rand::random::<[u8; 16]>())));
        self.nonces.lock().unwrap().insert(nonce.clone());
        nonce
    }

    fn consume_nonce(&self, nonce: &str) -> bool {
        self.nonces.lock().unwrap().remove(nonce)
    }

    fn www_authenticate_header(&self, nonce: &str) -> HeaderValue {
        HeaderValue::from_str(&format!(
            r#"Digest realm="{}", nonce="{}", algorithm=MD5, qop="auth""#,
            self.realm, nonce
        ))
        .unwrap()
    }
}

// ---------------------------------------------------------------------------
// Authorization header parser
// ---------------------------------------------------------------------------

struct DigestFields {
    username: String,
    realm: String,
    nonce: String,
    uri: String,
    nc: String,
    cnonce: String,
    qop: String,
    response: String,
}

fn parse_digest_header(value: &str) -> Option<DigestFields> {
    let value = value.strip_prefix("Digest ")?;

    let mut map = HashMap::new();
    for part in value.split(',') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            let k = k.trim();
            let v = v.trim().trim_matches('"');
            map.insert(k, v.to_string());
        }
    }

    Some(DigestFields {
        username: map.remove("username")?,
        realm: map.remove("realm")?,
        nonce: map.remove("nonce")?,
        uri: map.remove("uri")?,
        nc: map.remove("nc").unwrap_or_default(),
        cnonce: map.remove("cnonce").unwrap_or_default(),
        qop: map.remove("qop").unwrap_or_default(),
        response: map.remove("response")?,
    })
}

// ---------------------------------------------------------------------------
// Digest validation
// ---------------------------------------------------------------------------

fn validate(config: &DigestAuthConfig, req: &ServiceRequest, fields: &DigestFields) -> bool {
    if fields.realm != config.realm {
        return false;
    }

    let ha1 = match config.users.get(&fields.username) {
        Some(h) => h.clone(),
        None => return false,
    };

    let ha2 = format!(
        "{:x}",
        md5::compute(format!("{}:{}", req.method(), fields.uri))
    );

    let expected = if fields.qop == "auth" {
        format!(
            "{:x}",
            md5::compute(format!(
                "{}:{}:{}:{}:{}:{}",
                ha1, fields.nonce, fields.nc, fields.cnonce, fields.qop, ha2
            ))
        )
    } else {
        format!(
            "{:x}",
            md5::compute(format!("{}:{}:{}", ha1, fields.nonce, ha2))
        )
    };

    if expected != fields.response {
        return false;
    }

    config.consume_nonce(&fields.nonce)
}

// ---------------------------------------------------------------------------
// actix-web middleware
// ---------------------------------------------------------------------------

pub struct DigestAuth(pub Arc<DigestAuthConfig>);

impl<S, B> Transform<S, ServiceRequest> for DigestAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = DigestAuthService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(DigestAuthService {
            service,
            config: self.0.clone(),
        }))
    }
}

pub struct DigestAuthService<S> {
    service: S,
    config: Arc<DigestAuthConfig>,
}

impl<S, B> Service<ServiceRequest> for DigestAuthService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let config = self.config.clone();

        let authorized = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_digest_header)
            .map(|fields| validate(&config, &req, &fields))
            .unwrap_or(false);

        if !authorized {
            let nonce = config.issue_nonce();
            let www_auth = config.www_authenticate_header(&nonce);
            return Box::pin(async move {
                let response = HttpResponse::Unauthorized()
                    .insert_header((header::WWW_AUTHENTICATE, www_auth))
                    .json(serde_json::json!({
                        "result": false,
                        "code": ErrorCode::Unauthorized as u32,
                        "message": "Authentication required"
                    }));
                Ok(req.into_response(response).map_into_right_body())
            });
        }

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res.map_into_left_body())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> DigestAuthConfig {
        DigestAuthConfig::new(
            "Remote API".to_string(),
            vec![UserEntry { username: "admin".to_string(), password: "password".to_string() }],
        )
    }

    // DigestAuthConfig 生成時に HA1 = MD5(username:realm:password) が正しく計算されることを確認する
    #[test]
    fn ha1_is_md5_of_username_realm_password() {
        let config = make_config();
        let expected = format!("{:x}", md5::compute("admin:Remote API:password"));
        assert_eq!(config.users["admin"], expected);
    }

    // 発行した nonce は 1 回だけ消費でき、2 回目は拒否されることを確認する（リプレイ攻撃対策）
    #[test]
    fn nonce_can_be_issued_and_consumed_once() {
        let config = make_config();
        let nonce = config.issue_nonce();
        assert!(config.consume_nonce(&nonce));
        assert!(!config.consume_nonce(&nonce));
    }

    // 正しい形式の Authorization: Digest ヘッダーから各フィールドが正しく抽出されることを確認する
    #[test]
    fn parse_valid_digest_header() {
        let hdr = concat!(
            r#"Digest username="admin", realm="Remote API", "#,
            r#"nonce="abc123", uri="/api/v1/hello", "#,
            r#"nc=00000001, cnonce="xyz789", qop=auth, response="deadbeef""#
        );
        let f = parse_digest_header(hdr).unwrap();
        assert_eq!(f.username, "admin");
        assert_eq!(f.realm,    "Remote API");
        assert_eq!(f.nonce,    "abc123");
        assert_eq!(f.nc,       "00000001");
        assert_eq!(f.cnonce,   "xyz789");
        assert_eq!(f.qop,      "auth");
        assert_eq!(f.response, "deadbeef");
    }

    // Basic 認証ヘッダーは Digest ではないため None を返すことを確認する
    #[test]
    fn parse_rejects_basic_auth() {
        assert!(parse_digest_header("Basic dXNlcjpwYXNz").is_none());
    }

    // 必須フィールド (username, nonce, response 等) が欠けている場合に None を返すことを確認する
    #[test]
    fn parse_rejects_incomplete_digest_header() {
        assert!(parse_digest_header(r#"Digest realm="test""#).is_none());
    }
}
