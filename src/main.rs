use anyhow::{Context, Result, bail};
use axum::Router;
use dotenvy::dotenv;
use std::net::SocketAddr;
use tokio::{
    net::{TcpListener, UnixListener},
    signal,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

mod announce;
mod config;
mod error;
mod queue;
mod rate;
mod routes;
mod scheduler;
mod stats;
mod store;
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

    // Create router.
    let app = Router::new()
        .merge(routes::routes(tracker.clone()))
        .with_state(tracker.clone());

    // Ensure lock is dropped before axum::serve() is called otherwise
    // reloading config triggers a deadlock
    let config = tracker.config.load().clone();

    if let Some(path) = config.listening_unix_socket.to_owned() {
        // Create unix domain socket.
        let _ = tokio::fs::remove_file(&path).await;
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;

        let listener = UnixListener::bind(path.clone())?;

        // Start handling announces.
        println!("UNIT3D Announce has started.");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
    } else if let Some(ip) = config.listening_ip_address
        && let Some(port) = config.listening_port
    {
        // Create TCP socket.
        let addr = SocketAddr::from((ip, port));

        let listener = TcpListener::bind(addr).await?;

        // Start handling announces.
        println!("UNIT3D Announce has started.");

        let app = app.into_make_service_with_connect_info::<SocketAddr>();

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
    } else {
        bail!("Listener not configured.");
    }

    // Flush all remaining updates before shutting down.
    let max_flushes = 1000;
    let mut flushes = 0;

    while flushes < max_flushes && tracker.queues.are_not_empty() {
        tracker.queues.flush(&tracker).await;
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
