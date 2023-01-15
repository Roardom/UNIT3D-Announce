use axum::{routing::get, Router};
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
    let app =
        Router::with_state(tracker.clone()).route("/announce/:passkey", get(announce::announce));

    // Listening socket address.
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));

    // Start handling announces.
    println!("UNIT3D Announce has started.");

    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    // Flush updates one last time before shutting down.
    scheduler::handle(&tracker_clone2.clone()).await;

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
