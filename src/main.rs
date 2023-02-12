use axum::{
    routing::{get, put},
    Router,
};
use std::net::SocketAddr;
use tokio::signal;

use error::Error;

mod announce;
mod config;
mod error;
mod scheduler;
mod stats;
mod tracker;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // The Tracker struct keeps track of all state within the application.
    let tracker = tracker::Tracker::default().await?;

    // Clone the tracker so that it can be passed to the scheduler.
    let tracker_clone = tracker.clone();
    let tracker_clone2 = tracker.clone();

    // Starts scheduler to automate flushing updates
    // to database and inactive peer removal.
    let _handle = tokio::spawn(async move {
        scheduler::handle(&tracker_clone.clone()).await;
    });

    // Create router.
    let app = Router::new()
        .nest(
            "/announce",
            Router::new()
                .route("/:passkey", get(announce::announce))
                .nest(
                    &("/".to_string() + &tracker.config.apikey),
                    Router::new()
                        .route(
                            "/announce/torrents",
                            put(tracker::torrent::Map::upsert)
                                .delete(tracker::torrent::Map::destroy),
                        )
                        .route(
                            "/announce/users",
                            put(tracker::user::Map::upsert).delete(tracker::user::Map::destroy),
                        )
                        .route(
                            "/announce/blacklisted-agents",
                            put(tracker::blacklisted_agent::Set::upsert)
                                .delete(tracker::blacklisted_agent::Set::destroy),
                        )
                        .route(
                            "/announce/freeleech-tokens",
                            put(tracker::freeleech_token::Set::upsert)
                                .delete(tracker::freeleech_token::Set::destroy),
                        )
                        .route(
                            "/announce/personal-freeleech",
                            put(tracker::personal_freeleech::Set::upsert)
                                .delete(tracker::personal_freeleech::Set::destroy),
                        ),
                ),
        )
        .with_state(tracker.clone());

    // Listening socket address.
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));

    // Start handling announces.
    println!("UNIT3D Announce has started.");

    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    // Flush all remaining updates before shutting down.
    let max_flushes = 1000;
    let mut flushes = 0;

    while flushes < max_flushes
        && (tracker_clone2.history_updates.read().await.len() > 0
            || tracker_clone2.peer_updates.read().await.len() > 0
            || tracker_clone2.peer_deletions.read().await.len() > 0
            || tracker_clone2.torrent_updates.read().await.len() > 0
            || tracker_clone2.user_updates.read().await.len() > 0)
    {
        scheduler::flush(&tracker_clone2.clone()).await;
        flushes += 1;
    }

    if flushes == max_flushes {
        println!("Graceful shutdown failed");
    } else {
        println!("Graceful shutdown succeeded")
    }

    Ok(())
}

/// This future completes when shutdown signal is received.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("\nSignal received, starting graceful shutdown");
}
