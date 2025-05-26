use anyhow::{Context, Result};
use axum::Router;
use dotenvy::dotenv;
use tokio::{net::UnixListener, signal};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

mod announce;
mod config;
mod error;
mod rate;
mod routes;
mod scheduler;
mod stats;
mod tracker;
mod utils;
mod warning;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv().context(".env file not found.")?;

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=trace", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_level(false)
                .with_target(false),
        )
        .init();

    // The Tracker struct keeps track of all state within the application.
    let tracker = tracker::Tracker::default().await?;

    // Starts scheduler to automate flushing updates
    // to database and inactive peer removal.
    let _handle = tokio::spawn({
        let tracker = tracker.clone();

        async move {
            scheduler::handle(&tracker).await;
        }
    });

    let api_server = {
        // Create router.
        let api = Router::new()
            .merge(routes::api_routes(tracker.clone()))
            .with_state(tracker.clone());

        let listener =
            tokio::net::TcpListener::bind(tracker.config.read().listening_api_socket_address)
                .await
                .context("Unable to bind to unix socket.")?;

        axum::serve(listener, api).with_graceful_shutdown(shutdown_signal())
    };

    let announce_server = {
        // Create router.
        let announce = Router::new()
            .merge(routes::announce_routes(tracker.clone()))
            .with_state(tracker.clone());

        // Create unix domain socket.
        let path = tracker.config.read().listening_announce_socket.clone();

        let _ = tokio::fs::remove_file(&path).await;
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;
        let uds = UnixListener::bind(path.clone())?;

        axum::serve(uds, announce).with_graceful_shutdown(shutdown_signal())
    };

    // Start handling announces.
    println!("UNIT3D Announce has started.");

    let _ = tokio::join!(api_server, announce_server);

    // Flush all remaining updates before shutting down.
    let max_flushes = 1000;
    let mut flushes = 0;

    while flushes < max_flushes
        && (tracker.history_updates.lock().is_not_empty()
            || tracker.peer_updates.lock().is_not_empty()
            || tracker.torrent_updates.lock().is_not_empty()
            || tracker.user_updates.lock().is_not_empty())
    {
        scheduler::flush(&tracker).await;
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
