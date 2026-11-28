# Cambio: Intents para música aleatoria

## Intención
Permitir al usuario reproducir música aleatoria de tres formas:
1. Totalmente aleatoria: «pon música aleatoria»
2. Aleatoria de un artista: «pon música aleatoria de [artista]»
3. Aleatoria de una playlist: «pon música aleatoria de la playlist [nombre]»

## Alcance
- **Backend (Subsonic client)**: Añadir métodos `get_random_songs`, `get_artist_songs` (o reutilizar `getArtistTopSongs`), `get_playlist_songs` (ya existe `getPlaylist`).
- **Frontend (Alexa handlers)**: Nuevos intents `PlayRandomIntent`, `PlayRandomArtistIntent`, `PlayRandomPlaylistIntent` en `src/alexa.rs`.
- **Modelo de interacción**: Añadir intents y samples en `interaction_model.json` (locale es-ES).
- **Tests**: Tests TDD para deserialización, handlers y modelo.

## Impacto
- Sin cambios en la verificación RSA, endpoint `/webhook`, ni en `sessionAttributes.listing`.
- No se añade base de datos ni almacenamiento externo.
- Reutiliza la cola de reproducción existente (`start_queue`).

## Riesgos
- Subsonic `getRandomSongs` devuelve canciones aleatorias de toda la biblioteca; se debe limitar número (ej. 50).
- Para artista/playlist, se obtienen todas las canciones y se barajan en el lado del cliente (o se usa shuffle de Subsonic si existe).
- Los slots `Artist` y `Playlist` son `AMAZON.SearchQuery`; requieren normalización similar a `PlayMusicIntent`/`PlayPlaylistIntent`.
