//! Binary entry point for the Alexa-Navidrome bridge.

/// Starts the bridge using configuration from the environment.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    alexa_navidrome_bridge::run().await
}

#[cfg(test)]
mod tests {
    use alexa_navidrome_bridge::{app_context, build_router, middleware, Config, MAX_BODY_SIZE};
    use axum::body::{to_bytes, Body};
    use axum::extract::Request;
    use axum::http::header::CONTENT_TYPE;
    use axum::http::{HeaderMap, StatusCode};
    use axum::response::Response;
    use axum::Router;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    const CERT_URL: &str = "https://s3.amazonaws.com/echo.api/echo-api-cert.pem";

    fn test_config() -> Config {
        Config {
            navidrome_url: "http://127.0.0.1:1".into(),
            navidrome_user: "user".into(),
            navidrome_password: "password".into(),
            alexa_skill_id: None,
            port: 3000,
            skip_signature_verification: true,
        }
    }

    fn test_router(config: Config) -> Router {
        let skip = config.skip_signature_verification;
        build_router(app_context(config), skip)
    }

    fn launch_payload() -> Value {
        json!({
            "version": "1.0",
            "session": {
                "sessionId": "SessionId.abc",
                "application": { "applicationId": "amzn1.ask.skill.123" },
                "user": { "userId": "amzn1.ask.account.456" }
            },
            "context": {
                "System": {
                    "application": { "applicationId": "amzn1.ask.skill.123" },
                    "device": { "deviceId": "amzn1.ask.device.789" }
                }
            },
            "request": {
                "type": "LaunchRequest",
                "requestId": "amzn1.echo-api.request.1",
                "timestamp": "2026-01-01T00:00:00Z",
                "locale": "es-ES"
            }
        })
    }

    fn webhook_request(headers: HeaderMap, payload: Value) -> Request<Body> {
        let bytes = serde_json::to_vec(&payload).expect("serializable payload");
        let mut request = Request::builder()
            .method("POST")
            .uri("/webhook")
            .body(Body::from(bytes))
            .expect("valid request");
        *request.headers_mut() = headers;
        request
    }

    fn auth_headers(cert_url: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            "application/json".parse().expect("valid content type"),
        );
        headers.insert(
            middleware::CERT_CHAIN_URL_HEADER,
            cert_url.parse().expect("valid cert URL header"),
        );
        headers.insert(
            middleware::SIGNATURE_HEADER,
            "abc123".parse().expect("valid signature header"),
        );
        headers
    }

    async fn body_value(response: Response) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("readable body");
        serde_json::from_slice(&bytes).expect("JSON body")
    }

    #[tokio::test]
    async fn health_returns_ok_without_signature_headers() {
        let app = test_router(test_config());
        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .expect("valid request");

        let response = app.oneshot(request).await.expect("router response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_value(response).await, json!({ "status": "ok" }));
    }

    #[tokio::test]
    async fn webhook_rejects_missing_headers_even_when_crypto_is_skipped() {
        let app = test_router(test_config());
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            "application/json".parse().expect("valid content type"),
        );
        let request = webhook_request(headers, launch_payload());

        let response = app.oneshot(request).await.expect("router response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn webhook_accepts_launch_request_with_fake_headers_when_skipping() {
        let app = test_router(test_config());
        let request = webhook_request(auth_headers(CERT_URL), launch_payload());

        let response = app.oneshot(request).await.expect("router response");
        let status = response.status();
        let body = body_value(response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["version"], "1.0");
        assert!(body["response"]["outputSpeech"]["text"]
            .as_str()
            .is_some_and(|text| text.contains("Bienvenido")));
    }

    #[tokio::test]
    async fn webhook_rejects_invalid_certificate_url() {
        let app = test_router(test_config());
        let request = webhook_request(auth_headers("http://evil.com/cert"), launch_payload());

        let response = app.oneshot(request).await.expect("router response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn webhook_rejects_unexpected_skill_id() {
        let config = Config {
            alexa_skill_id: Some("amzn1.ask.skill.other".into()),
            ..test_config()
        };
        let app = test_router(config);
        let request = webhook_request(auth_headers(CERT_URL), launch_payload());

        let response = app.oneshot(request).await.expect("router response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn health_returns_ok_when_crypto_is_enabled() {
        let config = Config {
            skip_signature_verification: false,
            ..test_config()
        };
        let app = test_router(config);
        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .expect("valid request");

        let response = app.oneshot(request).await.expect("router response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn webhook_rejects_oversized_body_before_fetching_certificate() {
        let config = Config {
            skip_signature_verification: false,
            ..test_config()
        };
        let app = test_router(config);
        let mut payload = launch_payload();
        payload["request"]["type"] = json!("x".repeat(MAX_BODY_SIZE + 1));
        let request = webhook_request(auth_headers(CERT_URL), payload);

        let response = app.oneshot(request).await.expect("router response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
