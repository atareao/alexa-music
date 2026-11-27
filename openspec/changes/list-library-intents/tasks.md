# TDD Tasks — list-library-intents

## RED
- [ ] Test `get_album_list` leyendo `albumList2.album`.
- [ ] Test `get_artists` aplanando `artists.index[].artist[]`.
- [ ] Tests de `Session.attributes`, `speak_with_session`, `render_page`.
- [ ] Tests de handlers de listado y paginación.
- [ ] Test de `interaction_model.json` con los nuevos intents.

## GREEN
- [ ] Implementar structs y métodos en `src/subsonic.rs`.
- [ ] Implementar `ListingCursor` y helpers en `src/alexa.rs`.
- [ ] Implementar handlers y dispatch.
- [ ] Añadir intents y samples al modelo.

## REFACTOR
- [ ] `cargo clippy --all-targets -- -D warnings`.
- [ ] `cargo fmt -- --check`.

## RELEASE
- [ ] PRs a `development` y `main`.
- [ ] Tag y Release `v0.2.0`.
- [ ] Deploy en co1 y Build Model en consola Alexa.
