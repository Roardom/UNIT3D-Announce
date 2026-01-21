use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;

use anyhow::Result;
use tracing::info;

use crate::state::AppState;

use crate::model::passkey::Passkey;
use crate::store::user::User;

#[derive(Clone, Deserialize, Hash)]
pub struct APIInsertUser {
    pub id: u32,
    pub group_id: i32,
    pub passkey: String,
    pub new_passkey: Option<String>,
    pub can_download: bool,
    pub num_seeding: u32,
    pub num_leeching: u32,
    pub is_donor: bool,
    pub is_lifetime: bool,
}

pub async fn upsert(
    State(state): State<Arc<AppState>>,
    Json(user): Json<APIInsertUser>,
) -> StatusCode {
    info!("Received user: {}", user.id);
    if let Ok(passkey) = Passkey::from_str(&user.passkey) {
        info!("Inserting user with id {}.", user.id);
        let config = state.config.load();
        let old_user = state.stores.users.write().swap_remove(&user.id);
        let (receive_seed_list_rates, receive_leech_list_rates) = old_user
            .map(|user| (user.receive_seed_list_rates, user.receive_leech_list_rates))
            .unwrap_or_else(|| {
                (
                    config.user_receive_seed_list_rate_limits.clone(),
                    config.user_receive_leech_list_rate_limits.clone(),
                )
            });

        let new_passkey = if let Some(new_passkey) = &user.new_passkey {
            if let Ok(new_passkey) = Passkey::from_str(new_passkey) {
                state.stores.passkey2id.write().swap_remove(&passkey);
                new_passkey
            } else {
                return StatusCode::BAD_REQUEST;
            }
        } else {
            passkey
        };

        state.stores.users.write().insert(
            user.id,
            User {
                id: user.id,
                group_id: user.group_id,
                passkey: new_passkey,
                can_download: user.can_download,
                num_seeding: user.num_seeding,
                num_leeching: user.num_leeching,
                is_donor: user.is_donor,
                is_lifetime: user.is_lifetime,
                receive_seed_list_rates,
                receive_leech_list_rates,
            },
        );

        state.stores.passkey2id.write().insert(new_passkey, user.id);

        return StatusCode::OK;
    }

    StatusCode::BAD_REQUEST
}

#[derive(Clone, Deserialize, Hash)]
pub struct APIRemoveUser {
    pub id: u32,
    pub passkey: String,
}

pub async fn destroy(
    State(state): State<Arc<AppState>>,
    Json(user): Json<APIRemoveUser>,
) -> StatusCode {
    if let Ok(passkey) = Passkey::from_str(&user.passkey) {
        info!("Removing user with id {}.", user.id);

        state.stores.users.write().swap_remove(&user.id);
        state.stores.passkey2id.write().swap_remove(&passkey);

        return StatusCode::OK;
    }

    StatusCode::BAD_REQUEST
}

pub async fn show(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<User>, StatusCode> {
    state
        .stores
        .users
        .read()
        .get(&id)
        .map(|user| Json(user.clone()))
        .ok_or(StatusCode::NOT_FOUND)
}
