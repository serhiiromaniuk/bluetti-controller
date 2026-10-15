//! Server-sent events at `/events` (ROADMAP.md §8.2, §8.4 g). Built in
//! phase-5 step 11.

use axum::http::StatusCode;

/// `GET /events`: not built yet.
pub async fn events() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}
