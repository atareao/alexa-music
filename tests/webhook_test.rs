//! Integration tests for the public webhook router and signature verification.

use alexa_navidrome_bridge::middleware::{verify_signature_with_cert, AuthError};
use alexa_navidrome_bridge::{app_context, build_router, Config};
use axum::body::{to_bytes, Body};
use axum::extract::Request;
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Router;
use base64::{engine::general_purpose::STANDARD, Engine};
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::EncodePublicKey;
use rsa::signature::{SignatureEncoding, Signer};
use rsa::{RsaPrivateKey, RsaPublicKey};
use serde_json::{json, Value};
use sha2::Sha256;
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

fn test_router(skip_verification: bool) -> Router {
    let config = Config {
        skip_signature_verification: skip_verification,
        ..test_config()
    };
    build_router(app_context(config), skip_verification)
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

fn webhook_request(headers: HeaderMap, body: Vec<u8>) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri("/webhook")
        .body(Body::from(body))
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
        alexa_navidrome_bridge::middleware::CERT_CHAIN_URL_HEADER,
        cert_url.parse().expect("valid cert URL header"),
    );
    headers.insert(
        alexa_navidrome_bridge::middleware::SIGNATURE_HEADER,
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
    let response = test_router(false)
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("router response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_value(response).await, json!({ "status": "ok" }));
}

#[tokio::test]
async fn webhook_rejects_missing_signature_headers() {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        "application/json".parse().expect("valid content type"),
    );
    let body = serde_json::to_vec(&launch_payload()).expect("serializable payload");

    let response = test_router(true)
        .oneshot(webhook_request(headers, body))
        .await
        .expect("router response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn webhook_accepts_launch_request_with_fake_headers_when_skipping() {
    let body = serde_json::to_vec(&launch_payload()).expect("serializable payload");
    let response = test_router(true)
        .oneshot(webhook_request(auth_headers(CERT_URL), body))
        .await
        .expect("router response");
    let status = response.status();
    let value = body_value(response).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["version"], "1.0");
    assert!(value["response"]["outputSpeech"]["text"]
        .as_str()
        .is_some_and(|text| text.contains("Bienvenido")));
}

#[tokio::test]
async fn webhook_rejects_invalid_certificate_url() {
    let body = serde_json::to_vec(&launch_payload()).expect("serializable payload");
    let request = webhook_request(auth_headers("http://evil.com/cert"), body);

    let response = test_router(true)
        .oneshot(request)
        .await
        .expect("router response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn verify_signature_with_cert_accepts_signature_from_generated_rsa_key() {
    let private_key =
        RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).expect("generated RSA key pair");
    let public_key = RsaPublicKey::from(&private_key);
    let spki_der = public_key
        .to_public_key_der()
        .expect("encoded SPKI public key");
    let body = br#"{"request":{"type":"LaunchRequest"}}"#;

    let mut signed = Vec::with_capacity(body.len() + 10);
    signed.extend_from_slice(b"POST\n/webhook\n");
    signed.extend_from_slice(body);
    let signature = SigningKey::<Sha256>::new(private_key).sign(&signed);
    let signature_b64 = STANDARD.encode(signature.to_bytes());

    let result =
        verify_signature_with_cert(spki_der.as_ref(), &signature_b64, "POST", "/webhook", body);

    assert!(result.is_ok());
}

#[test]
fn verify_signature_with_cert_rejects_signature_over_different_body() {
    let private_key =
        RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).expect("generated RSA key pair");
    let public_key = RsaPublicKey::from(&private_key);
    let spki_der = public_key
        .to_public_key_der()
        .expect("encoded SPKI public key");
    let signature = SigningKey::<Sha256>::new(private_key).sign(b"POST\n/webhook\noriginal");
    let signature_b64 = STANDARD.encode(signature.to_bytes());

    let result = verify_signature_with_cert(
        spki_der.as_ref(),
        &signature_b64,
        "POST",
        "/webhook",
        b"tampered",
    );

    assert!(matches!(result, Err(AuthError::Verification(_))));
}

#[test]
fn verify_signature_with_cert_rejects_malformed_base64() {
    let private_key =
        RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).expect("generated RSA key pair");
    let spki_der = RsaPublicKey::from(&private_key)
        .to_public_key_der()
        .expect("encoded SPKI public key");

    let result = verify_signature_with_cert(spki_der.as_ref(), "%%%", "POST", "/webhook", b"{}");

    assert!(matches!(result, Err(AuthError::SignatureDecode(_))));
}
