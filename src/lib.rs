//! Alexa-Navidrome bridge library.
//!
//! The binary target is intentionally thin: all request handling lives here so
//! that integration tests can construct the same router with a test
//! configuration.

pub mod alexa;
pub mod app;
pub mod config;
pub mod middleware;
pub mod state;
pub mod subsonic;

pub use app::{app_context, build_router, health, webhook_handler, AppContext, MAX_BODY_SIZE};
pub use config::Config;

/// Initializes logging, loads configuration and serves the bridge until shutdown.
///
/// # Errors
///
/// Returns an error when configuration cannot be loaded, the TCP port cannot be
/// bound, or the HTTP server fails.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::from_env()?;
    let port = config.port;
    let skip_verification = config.skip_signature_verification;
    let router = build_router(app_context(config), skip_verification);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!(port, "listening for Alexa requests");
    axum::serve(listener, router).await?;
    Ok(())
}
