use std::sync::Arc;

use axum::{Json, extract::State};
use tracing::info;

use crate::{state::AppState, store::blacklisted_agent::Agent};

pub async fn upsert(State(state): State<Arc<AppState>>, Json(agent): Json<Agent>) {
    info!(
        "Inserting agent with peer_id_prefix {} ({:?}).",
        String::from_utf8_lossy(&agent.peer_id_prefix),
        agent.peer_id_prefix,
    );

    state.stores.agent_blacklist.write().insert(agent);
}

pub async fn destroy(State(state): State<Arc<AppState>>, Json(agent): Json<Agent>) {
    info!(
        "Removing agent with peer_id_prefix {} ({:?}).",
        String::from_utf8_lossy(&agent.peer_id_prefix),
        agent.peer_id_prefix,
    );

    state.stores.agent_blacklist.write().swap_remove(&agent);
}
