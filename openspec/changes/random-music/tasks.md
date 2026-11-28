# Tareas para random music

## RED: Crear tests de deserialización e intents nuevos
- [ ] Añadir test `model_declares_random_music_intents` (verifica que los nuevos intents aparecen en JSON).
- [ ] Test `handle_play_random` (falla antes de implementar handler).
- [ ] Test `handle_play_random_artist` y `handle_play_random_playlist`.

## GREEN: Implementación mínima
- [ ] Implementar métodos `get_random_songs`, `get_artist_songs`, `get_playlist_songs` en `src/subsonic.rs`.
- [ ] Implementar handlers `PlayRandomIntent`, `PlayRandomArtistIntent`, `PlayRandomPlaylistIntent` en `src/alexa.rs`.
- [ ] Actualizar `interaction_model.json` con los nuevos intents.

## REFACTOR: Linters y formateo
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo fmt -- --check`

## CONSOLIDAR
- [ ] Commit, PR a `development`, cerrar issue #52.
