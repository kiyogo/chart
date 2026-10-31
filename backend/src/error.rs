use axum::{
    Json,
    extract::rejection::{JsonRejection, PathRejection, QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// API error. The client only ever sees a short machine-readable code.
#[derive(Debug)]
pub enum AppError {
    BadRequest(&'static str),
    Unauthorized,
    Forbidden(&'static str),
    NotFound(&'static str),
    Conflict(&'static str),
    PayloadTooLarge,
    TooManyRequests,
    Unavailable,
    Internal,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    fn parts(&self) -> (StatusCode, &'static str) {
        match self {
            Self::BadRequest(c) => (StatusCode::BAD_REQUEST, c),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::Forbidden(c) => (StatusCode::FORBIDDEN, c),
            Self::NotFound(c) => (StatusCode::NOT_FOUND, c),
            Self::Conflict(c) => (StatusCode::CONFLICT, c),
            Self::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large"),
            Self::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.parts();
        (status, Json(json!({ "error": code }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed | sqlx::Error::Io(_) => {
                tracing::error!(error = %e, "database unavailable");
                Self::Unavailable
            }
            _ => {
                tracing::error!(error = %e, "database error");
                Self::Internal
            }
        }
    }
}

impl From<JsonRejection> for AppError {
    fn from(r: JsonRejection) -> Self {
        match r {
            JsonRejection::BytesRejection(_) if r.status() == StatusCode::PAYLOAD_TOO_LARGE => {
                Self::PayloadTooLarge
            }
            JsonRejection::MissingJsonContentType(_) => Self::BadRequest("content_type"),
            _ => Self::BadRequest("invalid_json"),
        }
    }
}

impl From<PathRejection> for AppError {
    fn from(_: PathRejection) -> Self {
        Self::BadRequest("invalid_path")
    }
}

impl From<QueryRejection> for AppError {
    fn from(_: QueryRejection) -> Self {
        Self::BadRequest("invalid_query")
    }
}

/// `Json` extractor that maps rejections to `AppError`.
#[derive(axum::extract::FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct ApiJson<T>(pub T);

#[derive(axum::extract::FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(AppError))]
pub struct ApiPath<T>(pub T);

#[derive(axum::extract::FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(AppError))]
pub struct ApiQuery<T>(pub T);
