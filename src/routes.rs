use std::sync::Arc;

use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{get, post, put},
};

use crate::{announce, config::Config, stats, store, tracker::Tracker};

pub fn routes(state: Arc<Tracker>) -> Router<Arc<Tracker>> {
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
                            put(store::torrent::Map::upsert).delete(store::torrent::Map::destroy),
                        )
                        .route("/torrents/{id}", get(store::torrent::Map::show))
                        .route(
                            "/users",
                            put(store::user::Map::upsert).delete(store::user::Map::destroy),
                        )
                        .route("/users/{id}", get(store::user::Map::show))
                        .route(
                            "/groups",
                            put(store::group::Map::upsert).delete(store::group::Map::destroy),
                        )
                        .route(
                            "/blacklisted-agents",
                            put(store::blacklisted_agent::Set::upsert)
                                .delete(store::blacklisted_agent::Set::destroy),
                        )
                        .route(
                            "/freeleech-tokens",
                            put(store::freeleech_token::Set::upsert)
                                .delete(store::freeleech_token::Set::destroy),
                        )
                        .route(
                            "/personal-freeleech",
                            put(store::personal_freeleech::Set::upsert)
                                .delete(store::personal_freeleech::Set::destroy),
                        )
                        .route(
                            "/featured-torrents",
                            put(store::featured_torrent::Set::upsert)
                                .delete(store::featured_torrent::Set::destroy),
                        )
                        .route("/stats", get(crate::stats::show))
                        .route("/config/reload", post(Config::reload)),
                ),
        )
        .layer(from_fn_with_state(state.clone(), stats::record_request))
}
