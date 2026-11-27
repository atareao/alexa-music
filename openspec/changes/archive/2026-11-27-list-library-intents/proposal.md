# Change: Intents para listar biblioteca musical

## Why
La skill solo puede reproducir música; no sabe responder a “qué álbumes tengo” ni “qué artistas tengo”.Issue #31.

## What Changes
- Añadir `ListAlbumsIntent`, `ListArtistsIntent`, `ListPlaylistsIntent` y `MoreListingIntent`.
- Añadir métodos Subsonic `getAlbumList2` y `getArtists`.
- Guardar un `ListingCursor` en `sessionAttributes` para paginar sin base de datos.
- Actualizar `interaction_model.json` y documentación.

## Impact
- `src/subsonic.rs`, `src/alexa.rs`, `interaction_model.json`, docs.
- No toca firma RSA, colas de reproducción ni playlists existentes.
- Requiere re-importar el modelo y hacer Build Model en la consola de Alexa.
