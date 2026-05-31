//! Backend-for-frontend: the axum HTTP surface and UI-shaped DTOs. It depends on
//! `application` (use cases) and `domain` only — never on the infrastructure
//! adapters. The audio data plane is taken via the `AudioStreamProvider` port
//! defined here, which the composition root wires to a concrete source.

mod audio;
mod dto;
mod error;
mod handlers;
mod state;
mod stream;

pub use audio::AudioStreamProvider;
pub use state::AppState;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

/// Build the application router: REST API + the live audio stream, with the
/// static touch UI served at `/` (SPA-style fallback). CORS is permissive to
/// match the old flask-cors behavior (only relevant for cross-origin dev).
pub fn router(state: AppState) -> Router {
    // The UI directory is a router concern; capture it before `state` is moved
    // into the handlers via `.with_state`.
    let web_root = state.web_root.clone();

    Router::new()
        .route("/status", get(handlers::status))
        .route("/list-casts", get(handlers::list_casts))
        .route("/refresh-casts", post(handlers::refresh_casts))
        .route("/start-cast/{uuid}", post(handlers::start_cast))
        .route("/stop-cast", post(handlers::stop_cast))
        .route("/set-volume/{volume}", post(handlers::set_volume))
        .route("/volume-up", post(handlers::volume_up))
        .route("/volume-down", post(handlers::volume_down))
        .route("/volume-mute", post(handlers::volume_mute))
        .route("/volume-unmute", post(handlers::volume_unmute))
        .route("/audio-stream", get(stream::audio_stream))
        .fallback_service(ServeDir::new(&web_root).append_index_html_on_directories(true))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
