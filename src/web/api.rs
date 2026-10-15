//! The JSON API (ROADMAP.md §8.2, §8.4 f). Built in phase-5 step 9.

use axum::http::StatusCode;

/// `GET /api/status`: not built yet.
pub async fn status() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// `GET /api/series`: not built yet.
pub async fn series() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// `GET /api/outages`: not built yet.
pub async fn outages() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// `GET /api/energy`: not built yet.
pub async fn energy() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}
