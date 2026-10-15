//! Embedded static assets at `/assets/{name}` (ROADMAP.md §8.4 i). Built in
//! phase-5 step 8.

use axum::http::StatusCode;

/// `GET /assets/{name}`: not built yet.
pub async fn asset() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}
