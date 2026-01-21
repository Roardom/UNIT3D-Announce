use std::sync::Arc;

use axum::{Json, extract::State};
use tracing::info;

use crate::{state::AppState, store::featured_torrent::FeaturedTorrent};

pub async fn upsert(State(state): State<Arc<AppState>>, Json(token): Json<FeaturedTorrent>) {
    info!(
        "Inserting featured torrent with torrent_id {}.",
        token.torrent_id
    );

    state.stores.featured_torrents.write().insert(token);
}

pub async fn destroy(State(state): State<Arc<AppState>>, Json(token): Json<FeaturedTorrent>) {
    info!(
        "Removing featured torrent with torrent_id {}.",
        token.torrent_id
    );

    state.stores.featured_torrents.write().swap_remove(&token);
}
