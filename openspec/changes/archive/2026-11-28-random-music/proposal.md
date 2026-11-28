# Cambio: Intents para música aleatoria

## Why
La skill solo puede reproducir música por artista, álbum o playlist concretos, pero no sabe responder a peticiones abiertas como «pon música aleatoria», «pon música aleatoria de [artista]» o «pon música aleatoria de la playlist [nombre]». Añadir estas tres formas de reproducción aleatoria aprovecha la biblioteca de Navidrome sin almacenamiento adicional.

## What Changes
- Nuevos intents `PlayRandomIntent`, `PlayRandomArtistIntent` y `PlayRandomPlaylistIntent` en `src/alexa.rs`.
- Métodos Subsonic `get_random_songs`, `get_artist_songs` (o reutilizar `getArtistTopSongs`) y `get_playlist_songs` (reutilizando `getPlaylist`).
- Handlers `handle_play_random`, `handle_play_random_artist` y `handle_play_random_playlist`, reutilizando `start_queue` y la bandera de shuffle existente.
- `interaction_model.json` (es-ES): nuevos intents, samples y slots `Artist`/`Playlist` (`AMAZON.SearchQuery`).
- Tests TDD de deserialización, handlers y modelo.

## Impact
- Sin cambios en la verificación RSA, endpoint `/webhook`, ni en `sessionAttributes.listing`.
- No se añade base de datos ni almacenamiento externo.
- Reutiliza la cola de reproducción existente (`start_queue`).

## Riesgos
- Subsonic `getRandomSongs` devuelve canciones aleatorias de toda la biblioteca; se debe limitar número (ej. 50).
- Para artista/playlist, se obtienen todas las canciones y se barajan en el lado del cliente (o se usa shuffle de Subsonic si existe).
- Los slots `Artist` y `Playlist` son `AMAZON.SearchQuery`; requieren normalización similar a `PlayMusicIntent`/`PlayPlaylistIntent`.
