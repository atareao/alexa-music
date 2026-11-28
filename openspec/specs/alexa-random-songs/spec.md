# alexa-random-songs Specification

## Purpose
TBD - created by archiving change random-music. Update Purpose after archive.

## Requirements

### Requirement: PlayRandomIntent
- **Name**: `PlayRandomIntent`
- **Samples**:
  - "pon música aleatoria"
  - "pon música aleatoria por favor"
  - "reproduce música aleatoria"
  - "toca algo aleatorio"
- **Slots**: Ninguno.
- **Behavior**: Llama al método `get_random_songs` del cliente Subsonic y reproduce la primera canción (o una lista aleatoria de N canciones). Usa la cola de reproducción existente.

#### Scenario: Reproducir música aleatoria
- GIVEN una biblioteca con canciones y un dispositivo con la skill activada
- WHEN el usuario dice «pon música aleatoria»
- THEN el sistema MUST llamar a `get_random_songs`, cargar hasta 50 canciones en `start_queue` y reproducir la primera.

#### Scenario: Sin canciones disponibles
- GIVEN una biblioteca sin canciones o una respuesta vacía de Subsonic
- WHEN el usuario dice «pon música aleatoria»
- THEN el sistema MUST responder con un mensaje claro al usuario y MUST NO iniciar reproducción.

### Requirement: PlayRandomArtistIntent
- **Name**: `PlayRandomArtistIntent`
- **Slots**:
  - `Artist` (tipo `AMAZON.SearchQuery`)
- **Samples**:
  - "pon música aleatoria de [artista]"
  - "pon música aleatoria de [nombre de artista]"
  - "reproduce música aleatoria de [artista]"
- **Behavior**:
  1. Obtiene el nombre del artista desde el slot.
  2. Llama a `get_artist_top_songs` (o `get_artist` + `get_songs`) para obtener una lista de canciones.
  3. Baraja la lista (shuffle) y reproduce la primera canción o una lista aleatoria.
- **Consideración**: Si el artista tiene muchas canciones, limitar a 50 o usar `get_artist_top_songs`.

#### Scenario: Artista con canciones
- GIVEN un artista con canciones en Navidrome
- WHEN el usuario dice «pon música aleatoria de [artista]»
- THEN el sistema MUST obtener sus canciones, barajarlas y reproducir una selección aleatoria mediante `start_queue`.

#### Scenario: Artista sin coincidencia
- GIVEN un slot `Artist` que no coincide con ningún artista
- WHEN el usuario pide música aleatoria de ese artista
- THEN el sistema MUST responder con un mensaje claro sin iniciar reproducción.

### Requirement: PlayRandomPlaylistIntent
- **Name**: `PlayRandomPlaylistIntent`
- **Slots**:
  - `Playlist` (tipo `AMAZON.SearchQuery`)
- **Samples**:
  - "pon música aleatoria de la playlist [nombre]"
  - "pon música aleatoria de la lista [nombre]"
  - "reproduce música aleatoria de la playlist [nombre]"
- **Behavior**:
  1. Obtiene la lista de playlists mediante `get_playlists`.
  2. Busca la playlist cuyo nombre coincida (case-insensitive) con el slot.
  3. Obtiene la lista de canciones (`get_playlist`).
  4. Baraja la lista y reproduce la primera canción o una lista aleatoria.
- **Consideración**: Usar `normalize_for_playlist_search` para coincidencia.

#### Scenario: Playlist encontrada
- GIVEN una playlist con canciones y el slot `Playlist` con su nombre
- WHEN el usuario dice «pon música aleatoria de la playlist [nombre]»
- THEN el sistema MUST resolver la playlist con `normalize_for_playlist_search`, obtener sus canciones, barajarlas y reproducir una selección aleatoria.

#### Scenario: Playlist no encontrada
- GIVEN un slot `Playlist` sin coincidencia (case-insensitive/normalizada)
- WHEN el usuario pide música aleatoria de esa playlist
- THEN el sistema MUST responder con un mensaje claro sin iniciar reproducción.

### Requirement: Samples en español
- Todas las frases deben ser en español (es-ES) y alineadas con el estilo de los intents existentes.

#### Scenario: Modelo de interacción es-ES
- GIVEN el `interaction_model.json` de locale es-ES
- WHEN se valida el modelo
- THEN todos los samples de los nuevos intents MUST estar en español y seguir el estilo de los intents existentes.

### Requirement: Requerimientos de backend
- **Método `get_random_songs`**:
  - Parámetros: `size: usize`, `offset: usize` (opcional).
  - Consulta: `type=random&size={size}&offset={offset}` (usar endpoint existente o crear).
  - Respuesta: `albumList2` con lista de canciones.
- **Método `get_artist_songs`** (si no existe):
  - Parámetros: `artist_id: String`.
  - Consulta: `type=artist&id={artist_id}&size={size}`.
  - Respuesta: lista de canciones del artista.
- **Método `get_playlist_songs`**:
  - Parámetros: `playlist_id: String`.
  - Consulta: `type=playlist&id={playlist_id}`.
  - Respuesta: lista de canciones de la playlist.

#### Scenario: Consulta de canciones aleatorias
- GIVEN un cliente Subsonic configurado contra Navidrome
- WHEN se llama a `get_random_songs` con `size` y `offset`
- THEN el sistema MUST enviar `type=random&size={size}&offset={offset}` y deserializar la respuesta `albumList2` en una lista de canciones.

#### Scenario: Consulta por artista o playlist
- GIVEN un `artist_id` o un `playlist_id` válidos
- WHEN se llama a `get_artist_songs` o `get_playlist_songs`
- THEN el sistema MUST devolver la lista de canciones correspondiente del artista o de la playlist.

### Requirement: Tests
- Tests TDD para:
  - Deserialización de intents y samples.
  - Handlers `handle_play_random`, `handle_play_random_artist`, `handle_play_random_playlist`.
  - Validación de que el modelo JSON incluye los nuevos intents y que los tests fallan antes de implementar.

#### Scenario: Cobertura de los nuevos intents
- GIVEN los tests escritos antes de la implementación (RED)
- WHEN se ejecuta la suite
- THEN MUST fallar hasta que el modelo declare los intents y los handlers estén implementados, y MUST pasar (GREEN) una vez implementados.

### Requirement: Consideraciones de rendimiento
- `get_random_songs` debe ser eficiente; limitar a 50 canciones por defecto.
- Barajar listas en memoria es aceptable; la lista de canciones suele ser pequeña.
- Los handlers deben manejar errores de Subsonic y devolver mensajes claros al usuario.

#### Scenario: Límite y errores
- GIVEN una biblioteca muy grande o un error de Subsonic
- WHEN el usuario pide música aleatoria
- THEN el sistema MUST limitar la selección a 50 canciones por defecto y MUST traducir el error a un mensaje claro para el usuario.

### Requirement: Reutilización de shuffle existente
- `AMAZON.ShuffleOnIntent`/`AMAZON.ShuffleOffIntent` ya existen y pueden reutilizarse para la aleatoriedad dentro de `start_queue`.
- `handle_shuffle` ya gestiona la bandera de shuffle en la cola; los nuevos intents pueden reutilizar esa lógica.

#### Scenario: Shuffle de la cola
- GIVEN una cola creada por uno de los nuevos intents
- WHEN el usuario activa o desactiva shuffle con los intents existentes
- THEN `handle_shuffle` MUST actualizar la bandera de shuffle de esa cola sin cambios adicionales en los nuevos handlers.
