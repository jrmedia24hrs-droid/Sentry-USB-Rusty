use std::fmt::Write;

use axum::http::{HeaderMap, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::{Embed, EmbeddedFile};

#[derive(Embed)]
#[folder = "static/"]
struct StaticFiles;

/// SPA fallback handler: serve static files or index.html for client-side
/// routing. Sets caching headers so repeat page loads don't re-download the
/// 1.2 MB JS bundle from a car parked outside on flaky WiFi:
///
///   /assets/* — Vite content-hashes these (e.g. `index-CThdLhPi.js`); the
///   bytes are immutable, so cache them forever.
///
///   index.html and other entry files — `no-cache` so a soft reload picks
///   up a new bundle after an OTA update without a hard refresh.
///
/// ETag is the first 16 bytes (hex) of the sha256 hash that rust-embed
/// pre-computes at compile time. If the client's `If-None-Match` matches,
/// return 304 Not Modified — the asset isn't re-sent over the wire.
pub async fn spa_handler(uri: Uri, headers: HeaderMap) -> Response {
    let path = uri.path().trim_start_matches('/');

    if let Some(file) = StaticFiles::get(path) {
        return serve_embedded(path, file, &headers);
    }

    match StaticFiles::get("index.html") {
        Some(file) => serve_embedded("index.html", file, &headers),
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

fn serve_embedded(path: &str, file: EmbeddedFile, req_headers: &HeaderMap) -> Response {
    let etag = etag_for(&file);
    let cache_control = cache_control_for(path);

    if let Some(if_none_match) = req_headers.get(header::IF_NONE_MATCH) {
        if if_none_match.as_bytes() == etag.as_bytes() {
            return (
                StatusCode::NOT_MODIFIED,
                [
                    (header::CACHE_CONTROL, cache_control.to_string()),
                    (header::ETAG, etag),
                ],
            )
                .into_response();
        }
    }

    let mime = mime_guess::from_path(path).first_or_octet_stream();
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime.as_ref().to_string()),
            (header::CACHE_CONTROL, cache_control.to_string()),
            (header::ETAG, etag),
        ],
        file.data,
    )
        .into_response()
}

fn etag_for(file: &EmbeddedFile) -> String {
    let hash = file.metadata.sha256_hash();
    let mut s = String::with_capacity(34);
    s.push('"');
    for b in &hash[..16] {
        let _ = write!(s, "{:02x}", b);
    }
    s.push('"');
    s
}

fn cache_control_for(path: &str) -> &'static str {
    if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    }
}
