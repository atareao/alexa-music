//! Integration tests for the public webhook router and signature verification.

use alexa_navidrome_bridge::middleware::{
    alexa_auth_headers, validate_alexa_headers, verify_signature_with_cert, AuthError,
    SignatureAlgorithm, SIGNATURE_256_HEADER, SIGNATURE_HEADER,
};
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
use sha1::Sha1;
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
fn signature_256_over_raw_body_is_accepted() {
    let private_key =
        RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).expect("generated RSA key pair");
    let spki_der = RsaPublicKey::from(&private_key)
        .to_public_key_der()
        .expect("encoded SPKI public key");
    let body = serde_json::to_vec(&launch_payload()).expect("serializable payload");
    let signature = SigningKey::<Sha256>::new(private_key).sign(&body);
    let mut headers = auth_headers(CERT_URL);
    headers.remove(SIGNATURE_HEADER);
    headers.insert(
        SIGNATURE_256_HEADER,
        STANDARD
            .encode(signature.to_bytes())
            .parse()
            .expect("valid signature header"),
    );

    validate_alexa_headers(&headers).expect("Signature-256 headers are valid");
    let (_, signature_b64, algorithm) =
        alexa_auth_headers(&headers).expect("valid authentication headers");
    assert_eq!(algorithm, SignatureAlgorithm::Sha256);

    let result = verify_signature_with_cert(spki_der.as_ref(), signature_b64, algorithm, &body);

    assert!(result.is_ok(), "raw-body signature must verify: {result:?}");
}

#[test]
fn legacy_signature_header_over_raw_body_is_accepted() {
    let private_key =
        RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).expect("generated RSA key pair");
    let spki_der = RsaPublicKey::from(&private_key)
        .to_public_key_der()
        .expect("encoded SPKI public key");
    let body = serde_json::to_vec(&launch_payload()).expect("serializable payload");
    let signature = SigningKey::<Sha1>::new(private_key).sign(&body);
    let mut headers = auth_headers(CERT_URL);
    headers.insert(
        SIGNATURE_HEADER,
        STANDARD
            .encode(signature.to_bytes())
            .parse()
            .expect("valid signature header"),
    );

    validate_alexa_headers(&headers).expect("legacy headers are valid");
    let (_, signature_b64, algorithm) =
        alexa_auth_headers(&headers).expect("valid authentication headers");
    assert_eq!(algorithm, SignatureAlgorithm::Sha1);

    let result = verify_signature_with_cert(spki_der.as_ref(), signature_b64, algorithm, &body);

    assert!(
        result.is_ok(),
        "legacy raw-body signature must verify: {result:?}"
    );
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
    let signature = SigningKey::<Sha256>::new(private_key).sign(body);
    let signature_b64 = STANDARD.encode(signature.to_bytes());

    let result = verify_signature_with_cert(
        spki_der.as_ref(),
        &signature_b64,
        SignatureAlgorithm::Sha256,
        body,
    );

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
    let signature = SigningKey::<Sha256>::new(private_key).sign(b"original");
    let signature_b64 = STANDARD.encode(signature.to_bytes());

    let result = verify_signature_with_cert(
        spki_der.as_ref(),
        &signature_b64,
        SignatureAlgorithm::Sha256,
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

    let result =
        verify_signature_with_cert(spki_der.as_ref(), "%%%", SignatureAlgorithm::Sha256, b"{}");

    assert!(matches!(result, Err(AuthError::SignatureDecode(_))));
}

#[test]
fn model_declares_listing_intents() {
    let model: Value =
        serde_json::from_str(include_str!("../interaction_model.json")).expect("valid model JSON");
    let intents = model["interactionModel"]["languageModel"]["intents"]
        .as_array()
        .expect("intents array");

    for name in [
        "ListAlbumsIntent",
        "ListArtistsIntent",
        "ListPlaylistsIntent",
        "MoreListingIntent",
    ] {
        assert!(
            intents.iter().any(|intent| intent["name"] == name),
            "missing intent {name}"
        );
    }
    for intent in intents {
        let name = intent["name"].as_str().unwrap_or_default();
        let samples = intent["samples"].as_array().map_or(0, Vec::len);
        assert!(
            !name.starts_with("AMAZON.") || samples == 0,
            "AMAZON intent {name} has {samples} samples"
        );
    }
}

#[test]
fn model_declares_biblioteca_navidrom_invocation_name() {
    let model_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("interaction_model.json");
    let model_json = std::fs::read_to_string(&model_path).expect("read interaction_model.json");
    let model: Value = serde_json::from_str(&model_json).expect("valid model JSON");

    assert_eq!(
        model["interactionModel"]["languageModel"]["invocationName"],
        "biblioteca navidrom"
    );
}
