//! The raw telemetry page `/telemetry` (ROADMAP.md §8.1, §8.4 k). Built in
//! phase-5 step 13.

use axum::http::StatusCode;

/// `GET /telemetry`: not built yet.
pub async fn page() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}
