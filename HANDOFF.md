# HANDOFF — Alexa-Navidrome Bridge

## Estado
- Release/tag `v0.2.0` publicado; imagen `ghcr.io/atareao/alexa-music:v0.2.0` disponible en GHCR. **Deploy en co1 COMPLETADO**: `alexabridge` Up (healthy) con `v0.2.0`.
- 101 tests en verde con `v0.2.0`; `cargo check`, `cargo clippy --all-targets -- -D warnings` y `cargo fmt -- --check` limpios; auditoría ✅ APROBADA.
- `v0.2.0` añade `ListAlbumsIntent`, `ListArtistsIntent`, `ListPlaylistsIntent` y `MoreListingIntent`, con paginación de 10 mediante `sessionAttributes.listing = {"kind": "...", "offset": N}`. Issue #31; PRs #32/#33.

## Arquitectura (resumen 3 líneas)
- Binario Rust (Axum/Tokio) + `src/lib.rs` público; puente HTTPS Alexa ASK ↔ Navidrome Subsonic/OpenSubsonic.
- Cola de reproducción en memoria por `deviceId` (`Arc<RwLock<HashMap>>`), sin BD externa; cursor de listados en `sessionAttributes.listing` (solo `kind` + `offset`, límite ~24 KB de Alexa).
- Firma Alexa RSA verificada sobre el body crudo (preferencia `Signature-256`, fallback legacy SHA-1), con caché de certificados TTL 24h; `SKIP_SIGNATURE_VERIFICATION=false` en producción.

## Despliegue y verificación (co1)
- Stack: `/home/lorenzo/docker/dockge/stacks/navidrome/compose.yaml`, servicio `alexabridge`; imagen actual `ghcr.io/atareao/alexa-music:v0.2.0`, Up (healthy). Backup previo al deploy: `compose.yaml.bak-pre-v020`. `navidrome` Up 4 days, mismo container ID y StartedAt → no fue reiniciado.
- Config: `SKIP_SIGNATURE_VERIFICATION=false`, `ALEXA_SKILL_ID` configurado. No copiar valores de `.env` a docs ni al repo.
- Endpoints: `https://alexa.territoriolinux.es/webhook` y `https://alexa.territoriolinux.es/health` → `{"status":"ok"}`.
- Skill Custom es-ES, invocation name `biblioteca musical`, endpoint con certificado de CA pública (Let's Encrypt), sin wildcard. Audio Player activado en *Build → Interfaces* y modelo recompilado; `supportedInterfaces` no se pone en `interaction_model.json`. Tras el deploy de `v0.2.0` queda pendiente re-importar `interaction_model.json`, ejecutar **Build Model** y probar los intents de listado.
- Verificación post-deploy: `/health` → `{"status":"ok"}` HTTP 200; POST sin firma a `/webhook` → HTTP 401 (verificación RSA activa); logs `listening for Alexa requests port=3000`, sin errores. Verificación funcional previa: logs con `dispatching Alexa request intent=Some("PlayPlaylistIntent")`, `loading playlist tracks playlist=clasica song_count=Some(11)`, `starting playback track_id=6tat8goav82QFNlVTIs7ng` y `playback started token=track-6tat8goav82QFNlVTIs7ng error=None`; simulador mostró `Reproduciendo la lista clasica.`
- Detalles: `docs/deploy-co1.md` y `docs/alexa-signature-playlists.md` (incluye la sección de intents de listado de `v0.2.0`).

## Pendiente / siguiente paso
1. Re-importar `interaction_model.json` en la consola Alexa, ejecutar **Build Model** y probar «qué álbumes tengo», «qué artistas tengo», «qué playlists tengo» y «dime más», verificando que el cursor `sessionAttributes.listing` conserva solo `kind` y `offset`.
2. Confirmación auditiva en un Echo físico con la misma cuenta: el simulador web no reproduce audio; los logs confirman `AudioPlayer.PlaybackStarted`. (Deploy y verificación HTTP completados; no queda nada más pendiente en co1.)

## Comandos útiles
- `cargo test` · `cargo check` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt -- --check`
- En co1: `curl -i https://alexa.territoriolinux.es/health` · `docker logs --tail 100 alexabridge`

## CI/CD y seguimiento
- CI: `.github/workflows/ci.yml` (fmt/clippy/test en PRs) y `.github/workflows/release.yml` (GHCR), ver `docs/cicd.md`.
- Toolchain fijada a 1.88.0; publicación multi-arch pospuesta (arm64 lento bajo QEMU).
- Issue #16; PRs #17, #18 (firma), #19/#20 (modelo AudioPlayer), #21/#22 (sanitize Query), #23/#24 (samples AMAZON), #25/#26 (sanitize playlist), #27/#28 (entry playlist).
- Issue #31; PRs #32/#33 (intents de listado, `v0.2.0`); detalle en `PLAN.md`, `plans/PLAN-003.md` y `openspec/changes/list-library-intents/`.
- PLAN-002 completado y archivado; PLAN-001 completado; PLAN-003 implementado y pendiente de cierre/archivado.
