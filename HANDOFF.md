# HANDOFF — Alexa-Navidrome Bridge

## Estado
- Release/tag anotado `v0.2.1` sobre `32fdef2`; imagen `ghcr.io/atareao/alexa-music:v0.2.1` (`sha256:47482b9daf...`) publicada por el workflow run `46048584195` (OK). **Deploy en co1 COMPLETADO**: `alexabridge` Up (healthy) con ese digest.
- 107 tests en verde (85+7+4+11) tras el merge; `cargo check`, `cargo clippy --all-targets -- -D warnings` y `cargo fmt -- --check` limpios; auditoría ✅ APROBADA.
- PR #43 y #45 **MERGED** en `development` (merge commits `b7b3f7e` y `32fdef2`); conflicto de `HANDOFF.md` resuelto en `af5ee45`. `development` HEAD `32fdef2`.
- `v0.2.0` añade `ListAlbumsIntent`, `ListArtistsIntent`, `ListPlaylistsIntent` y `MoreListingIntent`, con paginación de 10 mediante `sessionAttributes.listing = {"kind": "...", "offset": N}`. Issue #31; PRs #32/#33.

## Arquitectura (resumen 3 líneas)
- Binario Rust (Axum/Tokio) + `src/lib.rs` público; puente HTTPS Alexa ASK ↔ Navidrome Subsonic/OpenSubsonic.
- Cola de reproducción en memoria por `deviceId` (`Arc<RwLock<HashMap>>`), sin BD externa; cursor de listados en `sessionAttributes.listing` (solo `kind` + `offset`, límite ~24 KB de Alexa).
- Firma Alexa RSA verificada sobre el body crudo (preferencia `Signature-256`, fallback legacy SHA-1), con caché de certificados TTL 24h; `SKIP_SIGNATURE_VERIFICATION=false` en producción.

## Despliegue y verificación (co1)
- Stack: `/home/lorenzo/docker/dockge/stacks/navidrome/compose.yaml`, servicio `alexabridge`; imagen actual `ghcr.io/atareao/alexa-music:v0.2.1` (`sha256:47482b9daf...`), Up (healthy). Backup previo al deploy `v0.2.0`: `compose.yaml.bak-pre-v020`. `navidrome` Up 4 days, mismo container ID y StartedAt → no fue reiniciado.
- Config: `SKIP_SIGNATURE_VERIFICATION=false`, `ALEXA_SKILL_ID` configurado. No copiar valores de `.env` a docs ni al repo.
- Endpoints: `https://alexa.territoriolinux.es/webhook` y `https://alexa.territoriolinux.es/health` → `{"status":"ok"}`.
- Skill Custom es-ES, invocation name `servidor personal`, endpoint con certificado de CA pública (Let's Encrypt), sin wildcard. Audio Player activado en *Build → Interfaces* y modelo recompilado; `supportedInterfaces` no se pone en `interaction_model.json`. Queda registrado que el modelo fue re-importado y se ejecutó **Build Model**; los intents de listado quedaron verificados en Echo el 27-11-2026 (ver § Sin pendientes funcionales; limpieza documental completada).
- Verificación post-deploy: `/health` → `{"status":"ok"}` HTTP 200; POST sin firma a `/webhook` → HTTP 401 (verificación RSA activa); logs `listening for Alexa requests port=3000`, sin errores. Verificación funcional previa: logs con `dispatching Alexa request intent=Some("PlayPlaylistIntent")`, `loading playlist tracks playlist=clasica song_count=Some(11)`, `starting playback track_id=6tat8goav82QFNlVTIs7ng` y `playback started token=track-6tat8goav82QFNlVTIs7ng error=None`; simulador mostró `Reproduciendo la lista clasica.` Tras el reinicio con `v0.2.1`: `/health` HTTP 200, webhook sin firma → 401, logs sin `no playlist matched` ni panic.
- Detalles: `docs/deploy-co1.md` y `docs/alexa-signature-playlists.md` (incluye la sección de intents de listado de `v0.2.0`).

## Sin pendientes funcionales; limpieza documental completada
0. **Development:** en la consola Alexa, pestaña **Test**, poner *Skill testing is enabled in* = **Development** (ni Off ni Live; si hay otra versión en Live, desactivarla). Después, en la app Alexa (misma cuenta del Dot): *Más → Skills y juegos → Tus habilidades → En desarrollo* → **Habilitar** `servidor personal`. Sin esto la skill no está disponible en la app ni en el dispositivo.
1. Verificado 27-11-2026 en Echo con `v0.2.1`: `LaunchRequest` 18:00–18:03Z abriendo `servidor personal`, `PlayPlaylistIntent` 18:24:36Z → `clasica` → `starting playback`/`playback started`, `ListPlaylistsIntent` 18:25:45Z y 18:25:55Z → `MoreListingIntent` 18:26:01Z. Sin `401` ni panic; el Dot respondió «no hay más» porque no quedaban más playlists → fin de lista correcto.
2. Confirmación en Echo físico: verificado en Echo el 27-11-2026 18:24:36Z con `v0.2.1` (`PlayPlaylistIntent`, `starting playback`, sin `no playlist matched` ni 401). El simulador web no reproduce audio, pero los logs confirman `AudioPlayer.PlaybackStarted`.
3. Fix de acentos (issue #44, `PLAN-007`, commits `0da8a6a`/`12b1ac1`/`aeb00fb`) verificado en Echo el 27-11-2026 18:24:36Z con `v0.2.1`: «Alexa, abre servidor personal y reproduce la playlist clásica» → `PlayPlaylistIntent`, playlist `clasica` (11 canciones), `starting playback` y `playback started`; sin `no playlist matched`, sin `401` ni panic. Issue #44 cerrado; `PLAN-007` y el change OpenSpec archivados.
4. `PLAN-006` / issue #42 cerrado: PR #43 (`b7b3f7e`) y PR #45 (`32fdef2`) **MERGED** en `development`; conflicto de `HANDOFF.md` resuelto en `af5ee45`; PR #41 cerrado sin merge. Verificación en Echo del 27-11-2026 con `v0.2.1` completada (punto 1): invocación, reproducción y paginación («no hay más» → fin de lista correcto), sin `401`. `PLAN-006` y el change OpenSpec archivados; issues #38, #40 y #42 cerrados.

## Comandos útiles
- `cargo test` · `cargo check` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt -- --check`
- En co1: `curl -i https://alexa.territoriolinux.es/health` · `docker logs --tail 100 alexabridge`

## CI/CD y seguimiento
- CI: `.github/workflows/ci.yml` (fmt/clippy/test en PRs) y `.github/workflows/release.yml` (GHCR), ver `docs/cicd.md`.
- Toolchain fijada a 1.88.0; publicación multi-arch pospuesta (arm64 lento bajo QEMU).
- Issue #16; PRs #17, #18 (firma), #19/#20 (modelo AudioPlayer), #21/#22 (sanitize Query), #23/#24 (samples AMAZON), #25/#26 (sanitize playlist), #27/#28 (entry playlist).
- Issue #31; PRs #32/#33 (intents de listado, `v0.2.0`); detalle en `PLAN.md`, `plans/PLAN-003.md` y `openspec/changes/list-library-intents/`.
- PLAN-002, PLAN-003, PLAN-006 y PLAN-007 completados y archivados; PLAN-001 archivado en esta limpieza. Cambio `fix-alexa-signature-body` archivado; issues #7, #10, #13, #16 y #31 cerrados.
