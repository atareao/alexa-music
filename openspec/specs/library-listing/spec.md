# library-listing Specification

## Purpose
TBD - created by archiving change list-library-intents. Update Purpose after archive.

## Requirements

### Requirement: List library items
El sistema MUST responder a `ListAlbumsIntent`, `ListArtistsIntent` y `ListPlaylistsIntent` con los nombres disponibles en Navidrome, en páginas de 10.

#### Scenario: List albums
- GIVEN una biblioteca con álbumes
- WHEN el usuario dice “qué álbumes tengo”
- THEN el sistema MUST responder con hasta 10 nombres y un cursor en `sessionAttributes`.

#### Scenario: Empty library
- GIVEN una biblioteca sin álbumes
- WHEN el usuario dice “qué álbumes tengo”
- THEN el sistema MUST responder “No tengo álbumes en la biblioteca.”

#### Scenario: No artists
- GIVEN una biblioteca sin artistas
- WHEN el usuario dice “qué artistas tengo”
- THEN el sistema MUST responder “No tengo artistas en la biblioteca.”

### Requirement: Paginate listings
El sistema MUST permitir “dime más” mediante `MoreListingIntent`, usando el cursor de `sessionAttributes`.

#### Scenario: More items available
- GIVEN un cursor con offset 10 y más elementos
- WHEN el usuario dice “dime más”
- THEN el sistema MUST devolver los siguientes 10 nombres y actualizar el offset.

#### Scenario: No cursor
- GIVEN una sesión sin cursor
- WHEN el usuario dice “dime más”
- THEN el sistema MUST responder “Dime primero qué quieres listar.”

#### Scenario: No more items
- GIVEN un cursor con offset al final de la lista
- WHEN el usuario dice “dime más”
- THEN el sistema MUST responder “No hay más álbumes.” (o el tipo correspondiente).

### Requirement: Subsonic compatibility
El sistema MUST usar `getAlbumList2` para álbumes y `getArtists` para artistas, y MUST mantener `getPlaylists` para listas.

#### Scenario: Navidrome artists response
- GIVEN una respuesta OpenSubsonic con `artists.index[].artist[]`
- WHEN se llama a `get_artists`
- THEN se MUST devolver una lista plana de artistas.
