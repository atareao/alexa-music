//! Validation of Alexa `SignatureCertChainUrl` and signature headers.
//!
//! The structural checks (header presence, certificate URL shape and a
//! base64-looking signature) always run. Full RSA-SHA256 verification is
//! performed by [`verify_signature`] on the raw request body; callers that
//! already hold a certificate's SPKI public key can use the network-free
//! [`verify_signature_with_cert`] instead. The legacy `Signature` header uses
//! RSA-SHA1 over the same raw body.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::http::HeaderMap;
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{Client, Url};
use rsa::{pkcs1v15, pkcs8::DecodePublicKey, signature::Verifier, RsaPublicKey};
use sha1::Sha1;
use sha2::Sha256;
use thiserror::Error;

/// Name of the header carrying the URL of the signing certificate chain.
pub const CERT_CHAIN_URL_HEADER: &str = "SignatureCertChainUrl";
/// Name of the header carrying the base64-encoded request signature.
pub const SIGNATURE_HEADER: &str = "Signature";
/// Name of the preferred header carrying the base64-encoded RSA-SHA256 signature.
pub const SIGNATURE_256_HEADER: &str = "Signature-256";

/// Hashing algorithm used by the selected Alexa signature header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignatureAlgorithm {
    /// Legacy `Signature` header: RSA-SHA1 over the raw request body.
    Sha1,
    /// Preferred `Signature-256` header: RSA-SHA256 over the raw request body.
    Sha256,
}

const CERT_HOST: &str = "s3.amazonaws.com";
const CERT_PATH_PREFIX: &str = "/echo.api/";
const CERT_FETCH_TIMEOUT: Duration = Duration::from_secs(10);
const CERT_CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const CERT_CACHE_MAX: usize = 64;
const CERT_BEGIN: &str = "-----BEGIN CERTIFICATE-----";
const CERT_END: &str = "-----END CERTIFICATE-----";

type CertCache = Mutex<HashMap<String, (Instant, Vec<u8>)>>;

/// In-memory cache of certificate chains, keyed by their normalized S3 URL.
static CERT_CACHE: OnceLock<CertCache> = OnceLock::new();

/// Reasons an Alexa request can fail authentication.
#[derive(Debug, Error)]
pub enum AuthError {
    /// A required authentication header is absent.
    #[error("missing required header: {0}")]
    MissingHeader(&'static str),
    /// A required header is not valid ASCII text or is empty.
    #[error("invalid or empty header: {0}")]
    InvalidHeader(&'static str),
    /// The certificate chain URL is malformed or outside the allowed S3 bucket.
    #[error("invalid SignatureCertChainUrl")]
    InvalidCertChainUrl,
    /// The signature is empty or does not look like base64.
    #[error("invalid signature header")]
    InvalidSignature,
    /// The signing certificate could not be downloaded.
    #[error("failed to fetch signing certificate: {0}")]
    CertFetch(reqwest::Error),
    /// The signing certificate is not a valid PEM/DER X.509 certificate or RSA key.
    #[error("failed to parse signing certificate: {0}")]
    CertParse(String),
    /// The signing certificate is expired or not yet valid.
    #[error("signing certificate is expired or not yet valid")]
    InvalidCertValidity,
    /// The signature header is not valid base64.
    #[error("failed to decode signature: {0}")]
    SignatureDecode(base64::DecodeError),
    /// The RSA-SHA256 signature does not match the signed request.
    #[error("signature verification failed: {0}")]
    Verification(rsa::signature::Error),
}

/// Returns the certificate URL, signature and algorithm from Alexa's headers.
///
/// `Signature-256` (RSA-SHA256) is preferred; the legacy `Signature` header
/// (RSA-SHA1) is used only when `Signature-256` is absent. When both are
/// present, `Signature-256` wins even if the legacy value is malformed.
///
/// # Errors
///
/// Returns [`AuthError`] when a header is missing, non-UTF-8 or empty.
pub fn alexa_auth_headers(
    headers: &HeaderMap,
) -> Result<(&str, &str, SignatureAlgorithm), AuthError> {
    let cert_url = header_value(headers, CERT_CHAIN_URL_HEADER)?;
    let (signature, algorithm) = signature_values(headers)?;
    Ok((cert_url, signature, algorithm))
}

/// Selects the signature header and its algorithm.
fn signature_values(headers: &HeaderMap) -> Result<(&str, SignatureAlgorithm), AuthError> {
    if headers.contains_key(SIGNATURE_256_HEADER) {
        Ok((
            header_value(headers, SIGNATURE_256_HEADER)?,
            SignatureAlgorithm::Sha256,
        ))
    } else {
        Ok((
            header_value(headers, SIGNATURE_HEADER)?,
            SignatureAlgorithm::Sha1,
        ))
    }
}

/// Validates the structure of the Alexa authentication headers.
///
/// Header presence, certificate URL and signature format checks always apply;
/// callers must use [`verify_signature`] to check the cryptographic signature.
///
/// # Errors
///
/// Returns [`AuthError`] when a header is missing, a header value cannot be
/// decoded, the certificate URL is not an Amazon S3 `https` URL under
/// `/echo.api/`, or the signature is not base64-looking.
pub fn validate_alexa_headers(headers: &HeaderMap) -> Result<(), AuthError> {
    let (cert_url, signature, _) = alexa_auth_headers(headers)?;

    if !is_allowed_cert_url(normalized_cert_url(cert_url)) {
        return Err(AuthError::InvalidCertChainUrl);
    }

    if !signature
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
    {
        return Err(AuthError::InvalidSignature);
    }

    Ok(())
}

/// Verifies the Alexa RSA signature over the raw request body.
///
/// `algorithm` selects RSA-SHA256 for `Signature-256` and RSA-SHA1 for the
/// legacy `Signature` header. The signing certificate chain is downloaded from
/// `cert_url` with the given HTTP client and cached in memory by its normalized
/// URL. The first PEM certificate in the chain supplies the RSA public key.
///
/// # Errors
///
/// Returns [`AuthError`] for an invalid certificate URL or signature encoding,
/// a failed certificate download, an unparseable or out-of-date certificate, or
/// a signature that does not match the request.
pub async fn verify_signature(
    client: &Client,
    cert_url: &str,
    signature_b64: &str,
    algorithm: SignatureAlgorithm,
    body: &[u8],
) -> Result<(), AuthError> {
    // Re-check the URL here so direct callers cannot fetch an arbitrary host.
    // Query strings and fragments are discarded both here and for the cache key
    // so they cannot create unbounded cache entries or change the fetched URL.
    let cert_url = normalized_cert_url(cert_url);
    if !is_allowed_cert_url(cert_url) {
        return Err(AuthError::InvalidCertChainUrl);
    }

    // Decode before the network fetch so malformed input fails immediately.
    let signature = decode_signature(signature_b64)?;

    let pem = fetch_certificate(client, cert_url).await?;
    let der = first_certificate_der(&pem)?;
    let (_, certificate) = x509_parser::parse_x509_certificate(&der)
        .map_err(|err| AuthError::CertParse(err.to_string()))?;
    if !certificate.validity().is_valid() {
        return Err(AuthError::InvalidCertValidity);
    }

    verify_signature_decoded(certificate.public_key().raw, &signature, algorithm, body)
}

/// Verifies a signature against an already-obtained SPKI DER public key.
///
/// This is the network-free counterpart of [`verify_signature`]: it performs
/// the same base64 decoding and RSA verification over the raw body without
/// validating or fetching a certificate URL. The caller is responsible for
/// trusting `cert_spki_der`.
///
/// # Errors
///
/// Returns [`AuthError`] when the signature is not valid base64/PKCS#1 v1.5, the
/// SPKI DER key is not a valid RSA public key, or the signature does not match.
pub fn verify_signature_with_cert(
    cert_spki_der: &[u8],
    signature_b64: &str,
    algorithm: SignatureAlgorithm,
    body: &[u8],
) -> Result<(), AuthError> {
    let signature = decode_signature(signature_b64)?;
    verify_signature_decoded(cert_spki_der, &signature, algorithm, body)
}

/// Decodes and validates the size of a base64-encoded PKCS#1 v1.5 signature.
fn decode_signature(signature_b64: &str) -> Result<pkcs1v15::Signature, AuthError> {
    let signature_bytes = STANDARD
        .decode(signature_b64.trim())
        .map_err(AuthError::SignatureDecode)?;
    pkcs1v15::Signature::try_from(signature_bytes.as_slice()).map_err(AuthError::Verification)
}

/// Verifies a decoded signature using an SPKI DER RSA public key.
fn verify_signature_decoded(
    cert_spki_der: &[u8],
    signature: &pkcs1v15::Signature,
    algorithm: SignatureAlgorithm,
    body: &[u8],
) -> Result<(), AuthError> {
    let public_key = RsaPublicKey::from_public_key_der(cert_spki_der)
        .map_err(|err| AuthError::CertParse(err.to_string()))?;
    match algorithm {
        SignatureAlgorithm::Sha256 => pkcs1v15::VerifyingKey::<Sha256>::new(public_key)
            .verify(body, signature)
            .map_err(AuthError::Verification),
        SignatureAlgorithm::Sha1 => pkcs1v15::VerifyingKey::<Sha1>::new(public_key)
            .verify(body, signature)
            .map_err(AuthError::Verification),
    }
}

/// Drops query strings and fragments from a certificate URL.
fn normalized_cert_url(cert_url: &str) -> &str {
    let without_query = cert_url.split('?').next().unwrap_or(cert_url);
    without_query.split('#').next().unwrap_or(without_query)
}

/// Checks that the certificate URL is an S3 `https` URL under `/echo.api/`.
fn is_allowed_cert_url(cert_url: &str) -> bool {
    Url::parse(cert_url).ok().is_some_and(|url| {
        url.scheme().eq_ignore_ascii_case("https")
            && url
                .host_str()
                .is_some_and(|host| host.eq_ignore_ascii_case(CERT_HOST))
            && url
                .path()
                .get(..CERT_PATH_PREFIX.len())
                .is_some_and(|path| path.eq_ignore_ascii_case(CERT_PATH_PREFIX))
    })
}

/// Reads a fresh cached certificate, discarding expired entries.
fn cached_certificate(key: &str) -> Option<Vec<u8>> {
    let mut cache = cert_cache().lock().unwrap_or_else(|err| err.into_inner());
    let fresh = cache
        .get(key)
        .is_some_and(|(fetched_at, _)| fetched_at.elapsed() < CERT_CACHE_TTL);
    if fresh {
        cache.get(key).map(|(_, pem)| pem.clone())
    } else {
        cache.remove(key);
        None
    }
}

/// Caches a certificate, evicting expired or oldest entries when necessary.
fn cache_certificate(key: &str, pem: Vec<u8>) {
    let mut cache = cert_cache().lock().unwrap_or_else(|err| err.into_inner());
    cache.retain(|_, (fetched_at, _)| fetched_at.elapsed() < CERT_CACHE_TTL);
    if !cache.contains_key(key) && cache.len() >= CERT_CACHE_MAX {
        let oldest = cache
            .iter()
            .min_by_key(|(_, (fetched_at, _))| *fetched_at)
            .map(|(old_key, _)| old_key.clone());
        if let Some(old_key) = oldest {
            cache.remove(&old_key);
        }
    }
    cache.insert(key.to_owned(), (Instant::now(), pem));
}

fn cert_cache() -> &'static CertCache {
    CERT_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Fetches a certificate chain, caching successful downloads by normalized URL.
async fn fetch_certificate(client: &Client, cert_url: &str) -> Result<String, AuthError> {
    if let Some(pem) = cached_certificate(cert_url) {
        return String::from_utf8(pem).map_err(|err| AuthError::CertParse(err.to_string()));
    }

    // The mutex guard is dropped before the network request: no `std::sync`
    // lock is held across an `await`.
    let response = client
        .get(cert_url)
        .timeout(CERT_FETCH_TIMEOUT)
        .send()
        .await
        .map_err(AuthError::CertFetch)?;
    let pem = response
        .error_for_status()
        .map_err(AuthError::CertFetch)?
        .text()
        .await
        .map_err(AuthError::CertFetch)?;

    cache_certificate(cert_url, pem.as_bytes().to_vec());
    Ok(pem)
}

/// Extracts and decodes the first PEM certificate from a certificate chain.
fn first_certificate_der(pem: &str) -> Result<Vec<u8>, AuthError> {
    let after_begin = pem
        .split_once(CERT_BEGIN)
        .ok_or_else(|| AuthError::CertParse("no BEGIN CERTIFICATE marker".into()))?
        .1;
    let encoded = after_begin
        .split_once(CERT_END)
        .ok_or_else(|| AuthError::CertParse("no END CERTIFICATE marker".into()))?
        .0;
    let compact: String = encoded.chars().filter(|ch| !ch.is_whitespace()).collect();
    STANDARD
        .decode(compact.as_bytes())
        .map_err(|err| AuthError::CertParse(format!("invalid certificate base64: {err}")))
}

/// Reads a non-empty, UTF-8 header value.
fn header_value<'a>(headers: &'a HeaderMap, name: &'static str) -> Result<&'a str, AuthError> {
    let value = headers
        .get(name)
        .ok_or(AuthError::MissingHeader(name))?
        .to_str()
        .map_err(|_| AuthError::InvalidHeader(name))?
        .trim();

    if value.is_empty() {
        Err(AuthError::InvalidHeader(name))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::pkcs1v15::SigningKey;
    use rsa::pkcs8::EncodePublicKey;
    use rsa::signature::{SignatureEncoding, Signer};
    use rsa::RsaPrivateKey;

    fn headers(cert_url: &str, signature: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if !cert_url.is_empty() {
            headers.insert(
                CERT_CHAIN_URL_HEADER,
                cert_url.parse().expect("valid header"),
            );
        }
        if !signature.is_empty() {
            headers.insert(SIGNATURE_HEADER, signature.parse().expect("valid header"));
        }
        headers
    }

    const GOOD_URL: &str = "https://s3.amazonaws.com/echo.api/echo-api-cert.pem";

    #[test]
    fn accepts_amazon_cert_url() {
        let headers = headers(GOOD_URL, "abc123");

        assert!(validate_alexa_headers(&headers).is_ok());
    }

    #[test]
    fn accepts_uppercase_echo_api_path() {
        let headers = headers(
            "https://s3.amazonaws.com/ECHO.API/echo-api-cert.pem",
            "abc123",
        );

        assert!(validate_alexa_headers(&headers).is_ok());
    }

    #[test]
    fn rejects_requests_without_signature_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            CERT_CHAIN_URL_HEADER,
            GOOD_URL.parse().expect("valid header"),
        );

        assert!(matches!(
            validate_alexa_headers(&headers),
            Err(AuthError::MissingHeader(SIGNATURE_HEADER))
        ));
    }

    #[test]
    fn rejects_non_https_cert_url() {
        let headers = headers("http://s3.amazonaws.com/echo.api/cert.pem", "abc123");

        assert!(matches!(
            validate_alexa_headers(&headers),
            Err(AuthError::InvalidCertChainUrl)
        ));
    }

    #[test]
    fn rejects_foreign_certificate_host() {
        let headers = headers("https://evil.example/echo.api/cert.pem", "abc123");

        assert!(matches!(
            validate_alexa_headers(&headers),
            Err(AuthError::InvalidCertChainUrl)
        ));
    }

    #[test]
    fn rejects_cert_url_outside_echo_api_path() {
        let headers = headers("https://s3.amazonaws.com/other/cert.pem", "abc123");

        assert!(matches!(
            validate_alexa_headers(&headers),
            Err(AuthError::InvalidCertChainUrl)
        ));
    }

    #[test]
    fn rejects_malformed_cert_url() {
        let headers = headers("not a url", "abc123");

        assert!(matches!(
            validate_alexa_headers(&headers),
            Err(AuthError::InvalidCertChainUrl)
        ));
    }

    #[test]
    fn validates_signature_format() {
        let valid = headers(GOOD_URL, "abc123+/= ");
        let invalid = headers(GOOD_URL, "not base64!");

        assert!(validate_alexa_headers(&valid).is_ok());
        assert!(matches!(
            validate_alexa_headers(&invalid),
            Err(AuthError::InvalidSignature)
        ));
    }

    #[test]
    fn accepts_signature_256_header_as_preferred() {
        let mut headers = headers(GOOD_URL, "legacySignature");
        headers.insert(
            SIGNATURE_256_HEADER,
            "sha256Signature".parse().expect("valid header"),
        );

        let (_, signature, algorithm) =
            alexa_auth_headers(&headers).expect("valid authentication headers");

        assert_eq!(signature, "sha256Signature");
        assert_eq!(algorithm, SignatureAlgorithm::Sha256);
    }

    #[test]
    fn accepts_legacy_signature_header_as_fallback() {
        let headers = headers(GOOD_URL, "legacySignature");

        let (_, signature, algorithm) =
            alexa_auth_headers(&headers).expect("valid authentication headers");

        assert_eq!(signature, "legacySignature");
        assert_eq!(algorithm, SignatureAlgorithm::Sha1);
    }

    #[test]
    fn rejects_malformed_preferred_signature_without_falling_back() {
        let mut headers = headers(GOOD_URL, "legacySignature");
        headers.insert(SIGNATURE_256_HEADER, "not base64!".parse().expect("header"));

        assert!(matches!(
            validate_alexa_headers(&headers),
            Err(AuthError::InvalidSignature)
        ));
    }

    #[test]
    fn accepts_rsa_sha256_signature_over_raw_body() {
        let private_key =
            RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).expect("generated RSA key pair");
        let spki_der = RsaPublicKey::from(&private_key)
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

        assert!(result.is_ok(), "raw-body signature must verify: {result:?}");
    }

    #[test]
    fn accepts_rsa_sha1_signature_over_raw_body_for_legacy_header() {
        let private_key =
            RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).expect("generated RSA key pair");
        let spki_der = RsaPublicKey::from(&private_key)
            .to_public_key_der()
            .expect("encoded SPKI public key");
        let body = br#"{"request":{"type":"LaunchRequest"}}"#;
        let signature = SigningKey::<Sha1>::new(private_key).sign(body);
        let signature_b64 = STANDARD.encode(signature.to_bytes());

        let result = verify_signature_with_cert(
            spki_der.as_ref(),
            &signature_b64,
            SignatureAlgorithm::Sha1,
            body,
        );

        assert!(
            result.is_ok(),
            "legacy raw-body signature must verify: {result:?}"
        );
    }

    #[test]
    fn rejects_tampered_body_after_valid_raw_body_signature() {
        let private_key =
            RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).expect("generated RSA key pair");
        let spki_der = RsaPublicKey::from(&private_key)
            .to_public_key_der()
            .expect("encoded SPKI public key");
        let signed_body = br#"{"request":{"type":"LaunchRequest"}}"#;
        let signature = SigningKey::<Sha256>::new(private_key).sign(signed_body);
        let signature_b64 = STANDARD.encode(signature.to_bytes());
        let tampered_body = br#"{"request":{"type":"IntentRequest"}}"#;

        let result = verify_signature_with_cert(
            spki_der.as_ref(),
            &signature_b64,
            SignatureAlgorithm::Sha256,
            tampered_body,
        );

        assert!(matches!(result, Err(AuthError::Verification(_))));
    }

    #[test]
    fn normalizes_query_and_fragment_from_cert_url() {
        assert_eq!(
            normalized_cert_url(&format!("{GOOD_URL}?n=1#fragment")),
            GOOD_URL
        );
        assert_eq!(
            normalized_cert_url(&format!("{GOOD_URL}#fragment?n=1")),
            GOOD_URL
        );
    }

    #[test]
    fn cached_certificates_expire_after_ttl() {
        let key = "https://cache.test/expired.pem";
        let stale = Instant::now()
            .checked_sub(CERT_CACHE_TTL + Duration::from_secs(1))
            .expect("valid instant");
        cert_cache()
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .insert(key.to_owned(), (stale, b"stale".to_vec()));

        assert!(cached_certificate(key).is_none());
        assert!(!cert_cache()
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .contains_key(key));
    }

    #[test]
    fn certificate_cache_evicts_oldest_entry_when_full() {
        let base = "https://cache.test/entry-";
        for index in 0..CERT_CACHE_MAX {
            cache_certificate(&format!("{base}{index}"), vec![index as u8]);
        }
        cache_certificate(&format!("{base}overflow"), vec![255]);

        let cache = cert_cache().lock().unwrap_or_else(|err| err.into_inner());
        assert!(cache.len() <= CERT_CACHE_MAX);
        assert!(!cache.contains_key(&format!("{base}0")));
    }

    #[test]
    fn extracts_der_from_first_pem_certificate() {
        let pem = format!(
            "ignored text\n{CERT_BEGIN}\nAAECAw==\n{CERT_END}\n{CERT_BEGIN}\nAQIDBA==\n{CERT_END}\n"
        );

        assert_eq!(
            first_certificate_der(&pem).expect("decoded DER"),
            [0, 1, 2, 3]
        );
    }

    #[test]
    fn rejects_pem_without_certificate_markers() {
        assert!(matches!(
            first_certificate_der("not a certificate"),
            Err(AuthError::CertParse(_))
        ));
    }

    #[tokio::test]
    async fn rejects_invalid_signature_encoding_without_network() {
        let result = verify_signature(
            &Client::new(),
            GOOD_URL,
            "%%%",
            SignatureAlgorithm::Sha256,
            b"{}",
        )
        .await;

        assert!(matches!(result, Err(AuthError::SignatureDecode(_))));
    }

    #[tokio::test]
    async fn rejects_foreign_cert_url_without_network() {
        let result = verify_signature(
            &Client::new(),
            "https://evil.example/echo.api/cert.pem",
            "abc123",
            SignatureAlgorithm::Sha256,
            b"{}",
        )
        .await;

        assert!(matches!(result, Err(AuthError::InvalidCertChainUrl)));
    }

    #[tokio::test]
    async fn rejects_cert_url_with_query_and_foreign_host() {
        let result = verify_signature(
            &Client::new(),
            "https://evil.example/echo.api/cert.pem?n=1",
            "abc123",
            SignatureAlgorithm::Sha256,
            b"{}",
        )
        .await;

        assert!(matches!(result, Err(AuthError::InvalidCertChainUrl)));
    }
}
