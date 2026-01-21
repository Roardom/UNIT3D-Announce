use std::sync::Arc;

use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{get, post, put},
};

use crate::{announce, api, config::Config, state::AppState, stats};

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .nest(
            "/announce",
            Router::new()
                .route("/{passkey}", get(announce::announce))
                .nest(
                    "/health",
                    Router::new().route("/ping", get(|| async { "PONG" })),
                )
                .nest(
                    &("/".to_string() + &state.config.load().apikey),
                    Router::new()
                        .route(
                            "/torrents",
                            put(api::torrent::upsert).delete(api::torrent::destroy),
                        )
                        .route("/torrents/{id}", get(api::torrent::show))
                        .route("/users", put(api::user::upsert).delete(api::user::destroy))
                        .route("/users/{id}", get(api::user::show))
                        .route(
                            "/groups",
                            put(api::group::upsert).delete(api::group::destroy),
                        )
                        .route(
                            "/blacklisted-agents",
                            put(api::blacklisted_agent::upsert)
                                .delete(api::blacklisted_agent::destroy),
                        )
                        .route(
                            "/freeleech-tokens",
                            put(api::freeleech_token::upsert).delete(api::freeleech_token::destroy),
                        )
                        .route(
                            "/personal-freeleech",
                            put(api::personal_freeleech::upsert)
                                .delete(api::personal_freeleech::destroy),
                        )
                        .route(
                            "/featured-torrents",
                            put(api::featured_torrent::upsert)
                                .delete(api::featured_torrent::destroy),
                        )
                        .route("/stats", get(crate::stats::show))
                        .route("/config/reload", post(Config::reload)),
                ),
        )
        .layer(from_fn_with_state(state.clone(), stats::record_request))
}
