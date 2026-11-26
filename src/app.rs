//! HTTP application wiring shared by the binary and integration tests.
//!
//! Keeping the router in the library target makes it possible to exercise the
//! real middleware stack via `tower::ServiceExt::oneshot()` without starting a
//! network listener.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self as axum_middleware, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use tracing::warn;

use crate::alexa::{self, AlexaRequest, AlexaResponse};
use crate::config::Config;
use crate::middleware::{alexa_auth_headers, validate_alexa_headers, verify_signature};
use crate::state::AppState;
use crate::subsonic::SubsonicClient;

/// Maximum request body size accepted for signature verification.
pub const MAX_BODY_SIZE: usize = 1024 * 1024;

/// Dependencies shared by the `/webhook` handler.
#[derive(Clone)]
pub struct AppContext {
    /// Loaded service configuration.
    pub config: Config,
    /// Client used to query Navidrome.
    pub subsonic: SubsonicClient,
    /// Per-device playback queues.
    pub state: AppState,
}

/// Creates the shared dependencies from a loaded configuration.
pub fn app_context(config: Config) -> AppContext {
    let subsonic = SubsonicClient::new(
        config.navidrome_url.clone(),
        config.navidrome_user.clone(),
        config.navidrome_password.clone(),
    );
    let state: AppState = Arc::new(RwLock::new(HashMap::new()));

    AppContext {
        config,
        subsonic,
        state,
    }
}

/// Builds the router used by both the binary and the integration tests.
///
/// When `skip_signature_verification` is `true`, header structure is still
/// validated but the RSA signature and certificate URL fetch are skipped.
pub fn build_router(ctx: AppContext, skip_signature_verification: bool) -> Router {
    let client = reqwest::Client::new();
    let auth_layer = axum_middleware::from_fn(move |req: Request, next: Next| {
        let skip = skip_signature_verification;
        let client = client.clone();
        async move { verify_request(skip, &client, req, next).await }
    });

    Router::new()
        .route("/health", get(health))
        .route("/webhook", post(webhook_handler))
        .layer(auth_layer)
        .with_state(ctx)
}

/// Rejects requests with invalid Alexa headers or signatures before handlers.
///
/// `/health` is deliberately exempt so load balancers can probe the service.
/// When crypto is enabled the body is read for signature verification and then
/// restored so the `Json<AlexaRequest>` extractor can still consume it.
async fn verify_request(
    skip_verification: bool,
    client: &reqwest::Client,
    req: Request,
    next: Next,
) -> Response {
    if req.uri().path() == "/health" {
        return next.run(req).await;
    }

    if let Err(err) = validate_alexa_headers(req.headers()) {
        warn!(error = %err, path = %req.uri().path(), "rejecting unauthenticated request");
        return StatusCode::UNAUTHORIZED.into_response();
    }

    if skip_verification {
        return next.run(req).await;
    }

    let (cert_url, signature) = match alexa_auth_headers(req.headers()) {
        Ok((cert_url, signature)) => (cert_url.to_owned(), signature.to_owned()),
        Err(err) => {
            warn!(error = %err, "rejecting request without authentication headers");
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };
    let (parts, body) = req.into_parts();
    let body_bytes = match to_bytes(body, MAX_BODY_SIZE).await {
        Ok(bytes) => bytes,
        Err(err) => {
            warn!(error = %err, path = %parts.uri.path(), "rejecting unreadable request body");
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    if let Err(err) = verify_signature(
        client,
        &cert_url,
        &signature,
        parts.method.as_str(),
        parts.uri.path(),
        &body_bytes,
    )
    .await
    {
        warn!(error = %err, path = %parts.uri.path(), "rejecting request with invalid Alexa signature");
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let req = Request::from_parts(parts, Body::from(body_bytes));
    next.run(req).await
}

/// Liveness probe; intentionally requires no Alexa headers.
pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

/// Validates and dispatches an incoming Alexa request.
pub async fn webhook_handler(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(req): Json<AlexaRequest>,
) -> Result<Json<AlexaResponse>, StatusCode> {
    if let Err(err) = validate_alexa_headers(&headers) {
        warn!(error = %err, "webhook rejected due to invalid Alexa headers");
        return Err(StatusCode::UNAUTHORIZED);
    }

    if ctx
        .config
        .alexa_skill_id
        .as_deref()
        .is_some_and(|expected| req.application_id() != Some(expected))
    {
        warn!("webhook rejected due to unexpected Alexa skill ID");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let response = alexa::handle_request(&req, &ctx.subsonic, &ctx.state, &ctx.config).await;
    Ok(Json(response))
}
