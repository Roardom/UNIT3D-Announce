use anyhow::{Context, Result};
use axum::Router;
use dotenvy::dotenv;
use std::net::SocketAddr;
use tokio::signal;
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
        .merge(routes::routes(tracker.clone()))
        .with_state(tracker.clone());

    // Listening socket address.
    let addr = SocketAddr::from((
        tracker.config.read().listening_ip_address,
        tracker.config.read().listening_port,
    ));

    // Start handling announces.
    println!("UNIT3D Announce has started.");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();

    // Flush all remaining updates before shutting down.
    let max_flushes = 1000;
    let mut flushes = 0;

    while flushes < max_flushes
        && (tracker_clone2.history_updates.lock().is_not_empty()
            || tracker_clone2.peer_updates.lock().is_not_empty()
            || tracker_clone2.torrent_updates.lock().is_not_empty()
            || tracker_clone2.user_updates.lock().is_not_empty())
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
