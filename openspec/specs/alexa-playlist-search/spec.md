# alexa-playlist-search Specification

## Purpose
TBD - created by archiving change playlist-accent-normalization. Update Purpose after archive.

## Requirements

### Requirement: Normalización simétrica al buscar playlists
El sistema MUST comparar el nombre hablado con los nombres de playlists de Navidrome usando versiones normalizadas de ambas cadenas, para que `PlayPlaylistIntent` encuentre la playlist aunque las tildes difieran.

- `handle_play_playlist(query, playlists)`: para cada `playlist.name`, la comparación se hace entre versiones normalizadas de `query` y del nombre.
- Normalización: `to_lowercase()` y eliminación de diacritíticos en vocales españolas (`á`, `é`, `í`, `ó`, `ú`, `ü`) tras el lowercasing. `ñ`/`Ñ` se conservan como letra distinta; no se convierten en `n`.
- La comparación sigue siendo `contains` sobre las cadenas normalizadas.
- El prefijo hablado se sigue limpiando con `sanitize_search_query` antes de normalizar.
- Sin coincidencia: se mantiene el `WARN no playlist matched the spoken name` con el query original.

#### Scenario: acento en el nombre hablado
- Given una playlist `clasica` en Navidrome
- When el usuario dice «reproduce la playlist clásica» y llega `PlayPlaylistIntent` con `query="clásica"`
- Then la playlist se selecciona y empieza la reproducción sin el WARN de no coincidencia.

#### Scenario: acento en el nombre guardado
- Given una playlist `Clásicos` en Navidrome
- When el usuario dice «reproduce clasicos»
- Then la playlist se selecciona.

#### Scenario: mayúsculas y prefijos
- Given una playlist `Favoritas`
- When el usuario dice «pon música de FAVORITAS»
- Then se mantiene el comportamiento actual y la playlist se selecciona.

#### Scenario: ñ no se colapsa
- Given playlists `anio` y `año`
- When el usuario dice «año»
- Then no se selecciona `anio` por efecto de la normalización de `ñ`.

#### Scenario: sin coincidencia
- Given un query que no aparece en ningún nombre
- When no hay match
- Then se registra `no playlist matched the spoken name` y no se reproduce nada.

#### Scenario: verificación en Echo
- Given un Echo Dot con `servidor personal` habilitado
- When el usuario dice «Alexa, abre servidor personal y reproduce la playlist clásica»
- Then los logs de co1 muestran `PlayPlaylistIntent` y `starting playback`, sin `no playlist matched` ni `401`.
