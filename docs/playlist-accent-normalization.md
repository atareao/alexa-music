# Normalización de acentos en búsqueda de playlists

## Qué es
Fix de `PlayPlaylistIntent`: el nombre hablado de una playlist con tilde (`clásica`) ahora casa con el nombre guardado en Navidrome sin tilde (`clasica`), y viceversa.

## Evidencia
- Logs de co1, 18:00:48Z–18:03:44Z: el Echo Dot sí abría `servidor personal` (`LaunchRequest`) y despachaba `PlayPlaylistIntent`, sin `401`.
- Tres intentos fallaron con `WARN no playlist matched the spoken name query="clásica"`; la playlist de Navidrome se llama `clasica`.
- Causa: `src/alexa.rs` comparaba `playlist.name.to_lowercase().contains(&needle)` sin normalizar diacritíticos.

## Ficheros implicados
- `src/alexa.rs`: nueva función `normalize_for_playlist_search` y su uso simétrico en `handle_play_playlist`.
- `src/alexa.rs` (`#[cfg(test)]`): tests `handle_play_playlist_matches_accented_spoken_name` y `handle_play_playlist_matches_accented_stored_name`.
- `docs/playlist-accent-normalization.md`: este documento.
- `HANDOFF.md`: estado y verificación pendiente en el Echo.

## Cómo funciona
Se aplica `to_lowercase()` y se convierten `á é í ó ú ü` en `a e i o u u` tanto al query (tras `sanitize_search_query`) como al nombre de cada playlist. `ñ` se conserva como letra distinta para no confundir `año` con `anio`. La comparación sigue siendo `contains`; el `WARN` original sigue registrando el query hablado sin normalizar.

## Variables/config
Sin variables nuevas. No cambian el endpoint HTTPS, `ALEXA_SKILL_ID` ni la imagen desplegada (`v0.2.0`). El cambio requiere redesplegar `alexabridge` porque toca `src/`.

## Cómo habilitar/desplegar
1. Hacer merge del PR de `feature/playlist-accent-normalization` a `development`.
2. Redesplegar la imagen `alexabridge` con el nuevo código en co1.
3. En el Echo: «Alexa, abre servidor personal y reproduce la playlist clásica».
4. Revisar `docker logs --tail 100 alexabridge`: debe aparecer `PlayPlaylistIntent` y `starting playback`, sin `no playlist matched` ni `401`.

## Decisiones
- Normalización acotada a las vocales acentuadas del español; sin dependencias nuevas.
- `ñ` no se descompone: es una letra, no una tilde.
- No se renombran playlists en Navidrome.

## Follow-ups
- Confirmación en Echo físico tras el redespliegue.
- Cerrar el issue #44 y archivar `PLAN-007` y el change OpenSpec.
- `PLAN-006` sigue pendiente solo de confirmar «dime más» en el Dot.
