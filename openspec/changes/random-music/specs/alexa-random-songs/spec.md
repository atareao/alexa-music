# Especificaciones para intents de música aleatoria

## Requisitos de modelo de interacción

### 1. PlayRandomIntent
- **Name**: `PlayRandomIntent`
- **Samples**:
  - "pon música aleatoria"
  - "pon música aleatoria por favor"
  - "reproduce música aleatoria"
  - "toca algo aleatorio"
- **Slots**: Ninguno.
- **Behavior**: Llama al método `get_random_songs` del cliente Subsonic y reproduce la primera canción (o una lista aleatoria de N canciones). Usa la cola de reproducción existente.

### 2. PlayRandomArtistIntent
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

### 3. PlayRandomPlaylistIntent
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

### 4. Samples en español
- Todas las frases deben ser en español (es-ES) y alineadas con el estilo de los intents existentes.

### 5. Requerimientos de backend
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

### 6. Tests
- Tests TDD para:
  - Deserialización de intents y samples.
  - Handlers `handle_play_random`, `handle_play_random_artist`, `handle_play_random_playlist`.
  - Validación de que el modelo JSON incluye los nuevos intents y que los tests fallan antes de implementar.

### 7. Consideraciones de rendimiento
- `get_random_songs` debe ser eficiente; limitar a 50 canciones por defecto.
- Barajar listas en memoria es aceptable; la lista de canciones suele ser pequeña.
- Los handlers deben manejar errores de Subsonic y devolver mensajes claros al usuario.

### 8. Notas
- `AMAZON.ShuffleOnIntent`/`AMAZON.ShuffleOffIntent` ya existen y pueden reutilizarse para la aleatoriedad dentro de `start_queue`.
- `handle_shuffle` ya gestiona la bandera de shuffle en la cola; los nuevos intents pueden reutilizar esa lógica.

