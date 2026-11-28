# Tareas para random music

## RED: Crear tests de deserialización e intents nuevos
- [x] Añadir test `model_declares_random_music_intents` (verifica que los nuevos intents aparecen en JSON).
- [x] Test `handle_play_random` (falla antes de implementar handler).
- [x] Test `handle_play_random_artist` y `handle_play_random_playlist`.

## GREEN: Implementación mínima
- [x] Implementar métodos `get_random_songs`, `get_artist_songs`, `get_playlist_songs` en `src/subsonic.rs`.
- [x] Implementar handlers `PlayRandomIntent`, `PlayRandomArtistIntent`, `PlayRandomPlaylistIntent` en `src/alexa.rs`.
- [x] Actualizar `interaction_model.json` con los nuevos intents.

## REFACTOR: Linters y formateo
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo fmt -- --check`

## CONSOLIDAR
- [x] Commit, PR a `development`, cerrar issue #52.
