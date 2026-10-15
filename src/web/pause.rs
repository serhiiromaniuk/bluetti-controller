//! `POST /api/polling/pause` (ROADMAP.md §8.4 h): stops polling only, never
//! sends a frame to a station (golden rule #2). Built in phase-5 step 12.

use axum::http::StatusCode;

/// `POST /api/polling/pause`: not built yet.
pub async fn pause() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}
