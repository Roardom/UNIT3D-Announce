use std::sync::Arc;

use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{get, post, put},
};

use crate::{announce, config::Config, state::AppState, stats, store};

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
                            put(store::torrent::upsert).delete(store::torrent::destroy),
                        )
                        .route("/torrents/{id}", get(store::torrent::show))
                        .route(
                            "/users",
                            put(store::user::upsert).delete(store::user::destroy),
                        )
                        .route("/users/{id}", get(store::user::show))
                        .route(
                            "/groups",
                            put(store::group::upsert).delete(store::group::destroy),
                        )
                        .route(
                            "/blacklisted-agents",
                            put(store::blacklisted_agent::upsert)
                                .delete(store::blacklisted_agent::destroy),
                        )
                        .route(
                            "/freeleech-tokens",
                            put(store::freeleech_token::upsert)
                                .delete(store::freeleech_token::destroy),
                        )
                        .route(
                            "/personal-freeleech",
                            put(store::personal_freeleech::upsert)
                                .delete(store::personal_freeleech::destroy),
                        )
                        .route(
                            "/featured-torrents",
                            put(store::featured_torrent::upsert)
                                .delete(store::featured_torrent::destroy),
                        )
                        .route("/stats", get(crate::stats::show))
                        .route("/config/reload", post(Config::reload)),
                ),
        )
        .layer(from_fn_with_state(state.clone(), stats::record_request))
}
