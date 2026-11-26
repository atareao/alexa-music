# HANDOFF — Alexa-Navidrome Bridge

## Estado
- Implementación completa y verificada. Todos los objetivos de PLAN.md cumplidos.
- 77 tests en verde (`cargo test`), clippy `-D warnings` y fmt limpios.

## Arquitectura (resumen 3 líneas)
- Binario Rust (Axum/Tokio) + `src/lib.rs` público; puente HTTPS Alexa ASK ↔ Navidrome Subsonic v1.16.1.
- Cola de reproducción en memoria por `deviceId` (`Arc<RwLock<HashMap>>`), sin BD externa.
- Firma Alexa RSA-SHA256 verificada con caché de certs TTL 24h; `SKIP_SIGNATURE_VERIFICATION=true` solo para desarrollo.

## Pendiente / siguiente paso
- Desplegar: `Containerfile` → imagen → `alexabridge.container` en red `proxy.network` con Traefik.
- Importar `interaction_model.json` en Alexa Developer Console (JSON Editor, es-ES).
- Configurar `/etc/alexabridge.env` con NAVIDROME_* y ALEXA_SKILL_ID.

## Comandos útiles
- `cargo test` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt -- --check`
- `RUST_LOG=debug cargo run` (requiere variables de entorno)
