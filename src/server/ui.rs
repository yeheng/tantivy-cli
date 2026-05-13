use axum::{
    extract::Path,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct Assets;

pub async fn index() -> impl IntoResponse {
    match Assets::get("index.html") {
        Some(asset) => Html(asset.data).into_response(),
        None => (StatusCode::NOT_FOUND, "index.html not found in embedded assets").into_response(),
    }
}

pub async fn static_file(Path(path): Path<String>) -> impl IntoResponse {
    let path = path.trim_start_matches('/');

    // Try exact path first
    if let Some(asset) = Assets::get(path) {
        return build_response(path, &asset.data);
    }

    // Try with .html extension (for client-side router paths like /index/foo)
    let html_path = format!("{}.html", path);
    if let Some(asset) = Assets::get(&html_path) {
        return build_response(&html_path, &asset.data);
    }

    // SPA fallback: return index.html for unknown paths
    if let Some(asset) = Assets::get("index.html") {
        return build_response("index.html", &asset.data);
    }

    (StatusCode::NOT_FOUND, "not found").into_response()
}

fn build_response(path: &str, data: &[u8]) -> Response {
    let content_type = mime_guess::from_path(path).first_or_octet_stream();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type.as_ref())],
        data.to_vec(),
    )
        .into_response()
}
