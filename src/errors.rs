use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum ErrorCode {
    Unauthorized      = 1000,
    UnknownQueryParam = 1001,
    MissingParam      = 1002,
    InvalidParam      = 1003,
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
