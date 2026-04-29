use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde::Serialize;
use thiserror::Error;

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

#[derive(Serialize)]
struct ErrorBody {
    result: bool,
    code: ErrorCode,
    message: String,
}

#[derive(Debug, Error)]
#[allow(dead_code)]
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
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(ErrorBody {
            result: false,
            code: self.code(),
            message: self.to_string(),
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::body::to_bytes;

    // ErrorCode の各 variant が定義通りの数値にシリアライズされることを確認する
    #[test]
    fn error_code_numeric_values() {
        assert_eq!(serde_json::to_value(ErrorCode::Unauthorized).unwrap(),      1);
        assert_eq!(serde_json::to_value(ErrorCode::UnknownQueryParam).unwrap(), 2);
        assert_eq!(serde_json::to_value(ErrorCode::MissingParam).unwrap(),      3);
        assert_eq!(serde_json::to_value(ErrorCode::InvalidParam).unwrap(),      4);
    }

    // ApiError を返したとき HTTP 400 になり、JSON ボディに result/code/message が含まれることを確認する
    #[actix_web::test]
    async fn unknown_query_param_response() {
        let err = ApiError::UnknownQueryParam("foo".to_string());
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let bytes = to_bytes(resp.into_body()).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["result"], false);
        assert_eq!(body["code"],   ErrorCode::UnknownQueryParam as u32);
        assert_eq!(body["message"], "Unknown query parameter: foo");
    }
}
