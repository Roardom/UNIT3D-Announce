use std::sync::Arc;

use axum::{Json, extract::State};
use tracing::info;

use crate::{state::AppState, store::personal_freeleech::PersonalFreeleech};

pub async fn upsert(
    State(state): State<Arc<AppState>>,
    Json(personal_freeleech): Json<PersonalFreeleech>,
) {
    info!(
        "Inserting personal freeleech with user_id {}.",
        personal_freeleech.user_id
    );

    state
        .stores
        .personal_freeleeches
        .write()
        .insert(personal_freeleech);
}

pub async fn destroy(
    State(state): State<Arc<AppState>>,
    Json(personal_freeleech): Json<PersonalFreeleech>,
) {
    info!(
        "Removing personal freeleech with user_id {}.",
        personal_freeleech.user_id
    );

    state
        .stores
        .personal_freeleeches
        .write()
        .swap_remove(&personal_freeleech);
}
