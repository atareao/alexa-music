# PLAN-005 — Intents para música aleatoria

Issue: #52 — https://github.com/atareao/alexa-music/issues/52
Rama: `feature/random-music` (desde `development`)
Change OpenSpec: `openspec/changes/random-music/`

## Objetivo
Permitir al usuario reproducir música aleatoria de tres formas: totalmente aleatoria, de un artista, o de una playlist concreta, usando intents dedicados y el cliente Subsonic.

## Mapa arquitectónico
- `src/subsonic.rs`: nuevos métodos `get_random_songs`, `get_artist_songs`, `get_playlist_songs`; reutilizar `get_playlists`, `get_playlist`, `get_artist_top_songs`.
- `src/alexa.rs`: nuevos handlers `handle_play_random`, `handle_play_random_artist`, `handle_play_random_playlist`; despacho en `handle_request`.
- `interaction_model.json`: intents `PlayRandomIntent`, `PlayRandomArtistIntent`, `PlayRandomPlaylistIntent` con samples es-ES.
- `tests/webhook_test.rs`: validar modelo; tests de handlers en `src/alexa.rs`.
- Docs: `docs/random-music.md` y `HANDOFF.md`.

## Tareas (TDD)
- [ ] 1. RED: test que exija los tres intents en `interaction_model.json`; `cargo test` falla solo ahí.
- [ ] 2. RED: tests de `handle_play_random` (sin slot), `handle_play_random_artist` (slot Artist), `handle_play_random_playlist` (slot Playlist).
- [ ] 3. GREEN: implementar métodos Subsonic (`get_random_songs`, `get_artist_songs`, `get_playlist_songs`).
- [ ] 4. GREEN: implementar handlers y despacho.
- [ ] 5. GREEN: actualizar `interaction_model.json`.
- [ ] 6. REFACTOR: `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`.
- [ ] 7. Docs: `docs/random-music.md`, `HANDOFF.md`.
- [ ] 8. Cierre: PR a `development`, archivar change y plan, cerrar issue #52.

## Comandos de verificación
`cargo test` · `cargo check` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt -- --check`

## Criterios de aceptación
- Los tres intents existen en el modelo y el test de modelo pasa.
- `cargo test` en verde; sin regresiones en intents existentes.
- `PlayRandomIntent` reproduce una canción aleatoria de la biblioteca.
- `PlayRandomArtistIntent` reproduce canciones del artista indicado en orden aleatorio.
- `PlayRandomPlaylistIntent` reproduce canciones de la playlist indicada en orden aleatorio.
- Errores de Subsonic devuelven mensaje claro al usuario.
