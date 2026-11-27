# HANDOFF — Alexa-Navidrome Bridge

## Estado
- Implementación, despliegue y verificación final completados con `v0.1.9`.
- 88 tests en verde; `cargo check`, `cargo clippy --all-targets -- -D warnings` y `cargo fmt -- --check` limpios en el último cambio.
- Imagen desplegada en co1: `ghcr.io/atareao/alexa-music:v0.1.9`, healthy. Release/tag `v0.1.9` publicado en GitHub.

## Arquitectura (resumen 3 líneas)
- Binario Rust (Axum/Tokio) + `src/lib.rs` público; puente HTTPS Alexa ASK ↔ Navidrome Subsonic/OpenSubsonic.
- Cola de reproducción en memoria por `deviceId` (`Arc<RwLock<HashMap>>`), sin BD externa.
- Firma Alexa RSA verificada sobre el body crudo (preferencia `Signature-256`, fallback legacy SHA-1), con caché de certificados TTL 24h; `SKIP_SIGNATURE_VERIFICATION=false` en producción.

## Despliegue y verificación (co1)
- Stack: `/home/lorenzo/docker/dockge/stacks/navidrome/compose.yaml`, servicio `alexabridge`; `navidrome` sigue Up y no fue reiniciado.
- Config: `SKIP_SIGNATURE_VERIFICATION=false`, `ALEXA_SKILL_ID` configurado. No copiar valores de `.env` a docs ni al repo.
- Endpoints: `https://alexa.territoriolinux.es/webhook` y `https://alexa.territoriolinux.es/health` → `{"status":"ok"}`.
- Skill Custom es-ES, invocation name `biblioteca musical`, endpoint con certificado de CA pública (Let's Encrypt), sin wildcard. Audio Player activado en *Build → Interfaces* y modelo recompilado; `supportedInterfaces` no se pone en `interaction_model.json`.
- Verificación final: `/health` 200; POST sin headers → 401; logs con `dispatching Alexa request intent=Some("PlayPlaylistIntent")`, `loading playlist tracks playlist=clasica song_count=Some(11)`, `starting playback track_id=6tat8goav82QFNlVTIs7ng` y `playback started token=track-6tat8goav82QFNlVTIs7ng error=None`; simulador mostró `Reproduciendo la lista clasica.`
- Detalles: `docs/deploy-co1.md` y `docs/alexa-signature-playlists.md`.

## Pendiente / siguiente paso
- Único pendiente: confirmación auditiva en un Echo físico con la misma cuenta. El simulador web no reproduce audio; los logs confirman `AudioPlayer.PlaybackStarted`.

## Comandos útiles
- `cargo test` · `cargo check` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt -- --check`
- En co1: `curl -i https://alexa.territoriolinux.es/health` · `docker logs --tail 100 alexabridge`

## CI/CD y seguimiento
- CI: `.github/workflows/ci.yml` (fmt/clippy/test en PRs) y `.github/workflows/release.yml` (GHCR), ver `docs/cicd.md`.
- Toolchain fijada a 1.88.0; publicación multi-arch pospuesta (arm64 lento bajo QEMU).
- Issue #16; PRs #17, #18 (firma), #19/#20 (modelo AudioPlayer), #21/#22 (sanitize Query), #23/#24 (samples AMAZON), #25/#26 (sanitize playlist), #27/#28 (entry playlist).
- PLAN-002 completado y archivado; PLAN-001 completado.
