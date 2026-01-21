use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use tracing::info;

use crate::{state::AppState, store::group::Group};

#[derive(Clone, Deserialize, Hash)]
pub struct APIInsertGroup {
    pub id: i32,
    pub slug: String,
    pub download_slots: Option<u32>,
    pub is_immune: bool,
    pub is_freeleech: bool,
    pub is_double_upload: bool,
}

pub async fn upsert(
    State(state): State<Arc<AppState>>,
    Json(group): Json<APIInsertGroup>,
) -> StatusCode {
    info!("Inserting group with id {}.", group.id);

    state.stores.groups.write().insert(
        group.id,
        Group {
            id: group.id,
            slug: group.slug,
            download_slots: group.download_slots,
            is_immune: group.is_immune,
            download_factor: if group.is_freeleech { 0 } else { 100 },
            upload_factor: if group.is_double_upload { 200 } else { 100 },
        },
    );

    StatusCode::OK
}

#[derive(Clone, Deserialize, Hash)]
pub struct APIRemoveGroup {
    pub id: i32,
}

pub async fn destroy(
    State(state): State<Arc<AppState>>,
    Json(group): Json<APIRemoveGroup>,
) -> StatusCode {
    info!("Removing group with id {}.", group.id);

    state.stores.groups.write().swap_remove(&group.id);

    StatusCode::OK
}
