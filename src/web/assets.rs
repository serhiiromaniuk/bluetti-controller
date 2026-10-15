//! Embedded static assets at `/assets/{name}` (ROADMAP.md §8.4 i): compiled
//! into the binary, so the page works offline in an outage (golden rule #1).

use axum::extract::Path;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

const JS: &str = "text/javascript; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";

/// Every served asset: name → (bytes, content type).
static ASSETS: [(&str, &[u8], &str); 4] = [
    ("app.js", include_bytes!("../../assets/app.js"), JS),
    ("app.css", include_bytes!("../../assets/app.css"), CSS),
    (
        "uPlot.iife.min.js",
        include_bytes!("../../assets/uPlot.iife.min.js"),
        JS,
    ),
    (
        "uPlot.min.css",
        include_bytes!("../../assets/uPlot.min.css"),
        CSS,
    ),
];

/// `GET /assets/{name}`: the embedded bytes with a one-day cache, `404` for
/// an unknown name.
pub async fn asset(Path(name): Path<String>) -> Response {
    match ASSETS.iter().find(|(n, _, _)| *n == name) {
        Some((_, bytes, content_type)) => (
            [
                (header::CONTENT_TYPE, *content_type),
                (header::CACHE_CONTROL, "max-age=86400"),
            ],
            *bytes,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
