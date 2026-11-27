# PLAN-007 — Normalizar acentos en la búsqueda de playlists

Issue: #44 — https://github.com/atareao/alexa-music/issues/44
Rama: `feature/playlist-accent-normalization` (desde `development`)
Change OpenSpec: `openspec/changes/playlist-accent-normalization/`

## Objetivo
Arreglar que el Echo Dot reproduzca playlists cuyo nombre en Navidrome no lleva tilde. Los logs de co1 muestran que el skill sí se abre y despacha (`LaunchRequest`, `PlayPlaylistIntent`), pero `query="clásica"` no casa con la playlist `clasica`. La causa es la ausencia de normalización de diacritíticos en el matching.

## Mapa arquitectónico
- `src/alexa.rs`: `handle_play_playlist` (~`:801-829`); `sanitize_search_query` (~`:685-714`); tests en `#[cfg(test)]` (~`:1740`, `:1805`).
- Normalización simétrica: mismo tratamiento para el nombre hablado y para `playlist.name` antes de comparar.
- `docs/playlist-accent-normalization.md` y `HANDOFF.md`: documentación del fix y de la verificación.
- Sin cambios en `interaction_model.json`, intents, `sessionAttributes.listing`, endpoint ni contrato HTTP.

## Tareas (TDD)
- [x] 1. RED: añadir test que exija que `clásica` (hablado) case con `clasica` (Navidrome); `cargo test` debe fallar solo en ese caso.
- [x] 2. GREEN: normalizar diacritíticos de forma simétrica (vocales españolas á/é/í/ó/ú/ü tras `to_lowercase`; `ñ` se conserva como letra distinta); `cargo test` y `cargo check` en verde.
- [x] 3. REFACTOR: `cargo clippy --all-targets -- -D warnings` y `cargo fmt -- --check`.
- [x] 4. Docs: crear `docs/playlist-accent-normalization.md` y actualizar `HANDOFF.md` con la evidencia de co1 y las frases de prueba.
- [x] 5. Verificación externa: en el Echo, «Alexa, abre servidor personal y reproduce la playlist clásica» debe reproducir `clasica`; revisar `docker logs --tail 100 alexabridge` en co1 buscando `PlayPlaylistIntent`, `starting playback` y ausencia de `no playlist matched` / `401`.
- [x] 6. Cierre: PR a `development`, marcar tareas, renombrar a `plans/PLAN-007-ARCHIVED.md`, archivar el change y cerrar el issue #44.
- **Verificación (27-11-2026 18:24:36Z):** con la imagen `v0.2.1`, `PlayPlaylistIntent` cargó la playlist `clasica` (11 canciones) y registró `starting playback` / `playback started`; sin `no playlist matched`, sin `401` y sin panic.

## Comandos de verificación
`cargo test` · `cargo check` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt -- --check`

## Criterios de aceptación
- `clásica` y `clasica` casan; mayúsculas y prefijos del tipo «reproduce…» siguen funcionando.
- Sin coincidencia se sigue avisando con `no playlist matched the spoken name`.
- Sin regresiones en listados ni en paginación; 0 errores de firma.

## Nota
Este plan NO cierra `PLAN-006`: su tarea 6 sigue pendiente de confirmar «dime más» en el Dot (los `MoreListingIntent` registrados podrían venir del simulador). El rename a `servidor personal` está verificado: el Dot abre la skill sin `401`.
