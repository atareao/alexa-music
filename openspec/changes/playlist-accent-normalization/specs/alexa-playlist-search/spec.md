# Delta: `alexa-playlist-search-accents`

## Contratos
- `handle_play_playlist(query, playlists)`: para cada `playlist.name`, la comparación se hace entre versiones normalizadas de `query` y del nombre.
- Normalización: `to_lowercase()` y eliminación de diacritíticos en vocales españolas (`á`, `é`, `í`, `ó`, `ú`, `ü`) tras el lowercasing. `ñ`/`Ñ` se conservan como letra distinta; no se convierten en `n`.
- La comparación sigue siendo `contains` sobre las cadenas normalizadas.
- El prefijo hablado se sigue limpiando con `sanitize_search_query` antes de normalizar.
- Sin coincidencia: se mantiene el `WARN no playlist matched the spoken name` con el query original.

## Escenarios

### Escenario: acento en el nombre hablado
- Given una playlist `clasica` en Navidrome
- When el usuario dice «reproduce la playlist clásica» y llega `PlayPlaylistIntent` con `query="clásica"`
- Then la playlist se selecciona y empieza la reproducción sin el WARN de no coincidencia.

### Escenario: acento en el nombre guardado
- Given una playlist `Clásicos` en Navidrome
- When el usuario dice «reproduce clasicos»
- Then la playlist se selecciona.

### Escenario: mayúsculas y prefijos
- Given una playlist `Favoritas`
- When el usuario dice «pon música de FAVORITAS»
- Then se mantiene el comportamiento actual y la playlist se selecciona.

### Escenario: ñ no se colapsa
- Given playlists `anio` y `año`
- When el usuario dice «año»
- Then no se selecciona `anio` por efecto de la normalización de `ñ`.

### Escenario: sin coincidencia
- Given un query que no aparece en ningún nombre
- When no hay match
- Then se registra `no playlist matched the spoken name` y no se reproduce nada.

### Escenario: verificación en Echo
- Given un Echo Dot con `servidor personal` habilitado
- When el usuario dice «Alexa, abre servidor personal y reproduce la playlist clásica»
- Then los logs de co1 muestran `PlayPlaylistIntent` y `starting playback`, sin `no playlist matched` ni `401`.
