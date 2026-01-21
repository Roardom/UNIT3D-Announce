use std::sync::Arc;

use axum::{Json, extract::State};
use tracing::info;

use crate::{state::AppState, store::freeleech_token::FreeleechToken};

pub async fn upsert(State(state): State<Arc<AppState>>, Json(token): Json<FreeleechToken>) {
    info!(
        "Inserting freeleech token with user_id {} and torrent_id {}.",
        token.user_id, token.torrent_id
    );

    state.stores.freeleech_tokens.write().insert(token);
}

pub async fn destroy(State(state): State<Arc<AppState>>, Json(token): Json<FreeleechToken>) {
    info!(
        "Removing freeleech token with user_id {} and torrent_id {}.",
        token.user_id, token.torrent_id
    );

    state.stores.freeleech_tokens.write().swap_remove(&token);
}
