# TDD Tasks — fix-alexa-signature-body

## RED
- [x] Añadir test que firme un body con RSA-SHA256 sobre los bytes crudos y verifique que `verify_signature_with_cert` lo acepta.
- [x] Añadir test que modifique el body tras firmarlo y verifique rechazo.
- [x] Añadir test de integración con `Signature-256` sobre body crudo.
- [x] Ejecutar `cargo test` y confirmar que fallan los nuevos tests.

## GREEN
- [x] Eliminar `canonical_signed_bytes` y usar `body` directamente en `verify_signature_decoded`.
- [x] Añadir `SIGNATURE_256_HEADER` y seleccionar SHA-256/SHA-1 según cabecera.
- [x] Ejecutar `cargo test` hasta 100% verde y `cargo check`.

## REFACTOR
- [x] `cargo clippy --all-targets -- -D warnings` y `cargo fmt -- --check`.

## DEPLOY
- [ ] Publicar imagen `v0.1.1` en GHCR vía tag + GitHub Release.
- [ ] Actualizar el stack de co1 a `v0.1.1` y recrear solo `alexabridge`.
- [ ] Verificar con Alexa real: sin 401, `starting playback` en logs.
