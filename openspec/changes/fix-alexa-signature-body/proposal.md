# Change: Verificar firma Alexa sobre el body crudo

## Why
Con `SKIP_SIGNATURE_VERIFICATION=false` en co1, las peticiones reales de Alexa devuelven 401 con `signature verification failed: signature error: verification error`. La causa es que `canonical_signed_bytes()` construye `METHOD\nPATH\nBODY`, pero Alexa firma RSA-SHA256 sobre **el body HTTP completo** (confirmado en la documentación oficial "Host a Custom Skill as a Web Service" y en la implementación de referencia `mreinstein/alexa-verifier`: `crypto.createVerify('RSA-SHA256').update(requestBody)`).

## What Changes
- Reemplazar el dato firmado en `src/middleware.rs` por el body crudo (`body`), eliminando `canonical_signed_bytes`.
- Aceptar `Signature-256` como cabecera preferida (SHA-256 del body) y mantener `Signature` por compatibilidad (SHA-1 del body).
- Actualizar los tests unitarios y de integración que asumían la cadena canónica antigua.
- Publicar nueva imagen `v0.1.1` y desplegarla en co1.

## Impact
- `src/middleware.rs`: `verify_signature_decoded`, constantes de cabeceras, `validate_alexa_headers`.
- `src/app.rs`: selección de cabecera de firma.
- `tests/webhook_test.rs`: tests RSA con firma real sobre body.
- Seguridad: se corrige un fallo que rechazaba todo el tráfico legítimo de Alexa; no se relaja la verificación.
