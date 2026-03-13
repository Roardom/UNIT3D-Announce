use crate::config::Socket;
use anyhow::{Context, Result};
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
mod api;
mod config;
mod error;
mod model;
mod queue;
mod rate;
mod routes;
mod scheduler;
mod state;
mod stats;
mod store;
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

    // The state struct keeps track of all state within the application.
    let state = state::AppState::default().await?;

    // Starts scheduler to automate flushing updates
    // to database and inactive peer removal.
    let _handle = tokio::spawn({
        let state = state.clone();

        async move {
            scheduler::handle(&state).await;
        }
    });

    // Create router.
    let app = Router::new()
        .merge(routes::routes(state.clone()))
        .with_state(state.clone());

    // Ensure lock is dropped before axum::serve() is called otherwise
    // reloading config triggers a deadlock
    let config = state.config.load().clone();

    match &config.http_addr {
        Socket::Unix(path) => {
            // Create unix domain socket.
            let _ = tokio::fs::remove_file(&path).await;
            tokio::fs::create_dir_all(path.parent().unwrap()).await?;

            let listener = UnixListener::bind(path.clone())?;

            // Start handling announces.
            println!("UNIT3D Announce has started.");

            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await?;
        }
        Socket::Tcp(addr) => {
            let listener = TcpListener::bind(addr).await?;

            // Start handling announces.
            println!("UNIT3D Announce has started.");

            let app = app.into_make_service_with_connect_info::<SocketAddr>();

            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await?;
        }
    }

    // Flush all remaining updates before shutting down.
    let max_flushes = 1000;
    let mut flushes = 0;

    while flushes < max_flushes && state.queues.are_not_empty() {
        state.queues.flush(&state).await;
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
