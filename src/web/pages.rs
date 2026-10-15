//! The server-rendered status page `/` (ROADMAP.md §8.1, §8.3, §8.4 j).
//! Built in phase-5 step 10.

use axum::http::StatusCode;

/// `GET /`: not built yet.
pub async fn status() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}
