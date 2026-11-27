# TDD Tasks — list-library-intents

## RED
- [x] Test `get_album_list` leyendo `albumList2.album`.
- [x] Test `get_artists` aplanando `artists.index[].artist[]`.
- [x] Tests de `Session.attributes`, `speak_with_session`, `render_page`.
- [x] Tests de handlers de listado y paginación.
- [x] Test de `interaction_model.json` con los nuevos intents.

## GREEN
- [x] Implementar structs y métodos en `src/subsonic.rs`.
- [x] Implementar `ListingCursor` y helpers en `src/alexa.rs`.
- [x] Implementar handlers y dispatch.
- [x] Añadir intents y samples al modelo.

## REFACTOR
- [x] `cargo clippy --all-targets -- -D warnings`.
- [x] `cargo fmt -- --check`.

## RELEASE
- [x] PRs a `development` y `main`.
- [x] Tag y Release `v0.2.0`.
- [x] Deploy en co1 y Build Model en consola Alexa.

## Resultado

- Release `v0.2.0` publicada con PRs #32/#33 y docs #34/#35.
- 101 tests verdes.
- `cargo check`, `cargo clippy --all-targets -- -D warnings` y `cargo fmt -- --check` limpios.
- Auditoría ✅ APROBADA.
- Imagen `ghcr.io/atareao/alexa-music:v0.2.0`.
- Deploy en co1 en curso.
