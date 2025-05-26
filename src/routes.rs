use std::sync::Arc;

use axum::{
    middleware::from_fn_with_state,
    routing::{get, post, put},
    Router,
};

use crate::{
    announce,
    config::Config,
    stats,
    tracker::{self, Tracker},
};

pub fn announce_routes(state: Arc<Tracker>) -> Router<Arc<Tracker>> {
    Router::new()
        .route("/announce/{passkey}", get(announce::announce))
        .layer(from_fn_with_state(state.clone(), stats::record_request))
}

pub fn api_routes(state: Arc<Tracker>) -> Router<Arc<Tracker>> {
    Router::new().nest(
        &("/announce/".to_string() + &state.config.read().apikey),
        Router::new()
            .route(
                "/torrents",
                put(tracker::torrent::Map::upsert).delete(tracker::torrent::Map::destroy),
            )
            .route("/torrents/{id}", get(tracker::torrent::Map::show))
            .route(
                "/users",
                put(tracker::user::Map::upsert).delete(tracker::user::Map::destroy),
            )
            .route("/users/{id}", get(tracker::user::Map::show))
            .route(
                "/groups",
                put(tracker::group::Map::upsert).delete(tracker::group::Map::destroy),
            )
            .route(
                "/blacklisted-agents",
                put(tracker::blacklisted_agent::Set::upsert)
                    .delete(tracker::blacklisted_agent::Set::destroy),
            )
            .route(
                "/freeleech-tokens",
                put(tracker::freeleech_token::Set::upsert)
                    .delete(tracker::freeleech_token::Set::destroy),
            )
            .route(
                "/personal-freeleech",
                put(tracker::personal_freeleech::Set::upsert)
                    .delete(tracker::personal_freeleech::Set::destroy),
            )
            .route(
                "/featured-torrents",
                put(tracker::featured_torrent::Set::upsert)
                    .delete(tracker::featured_torrent::Set::destroy),
            )
            .route("/stats", get(crate::stats::show))
            .route("/config/reload", post(Config::reload)),
    )
}
