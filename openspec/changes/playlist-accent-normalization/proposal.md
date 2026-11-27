# Cambio: normalizar acentos al buscar playlists

## Intención
Hacer que `PlayPlaylistIntent` encuentre playlists de Navidrome aunque el nombre hablado lleve tilde y el nombre guardado no (o al revés). Evidencia: co1 registró `no playlist matched the spoken name query="clásica"` mientras la playlist es `clasica`.

## Alcance
- `src/alexa.rs`: normalización simétrica en `handle_play_playlist` (~`:801-829`).
- Tests RED del caso con acento.
- Documentación y `HANDOFF.md`.

## Impacto
- No cambia el invocation name, los intents, `sessionAttributes.listing`, el endpoint ni la imagen desplegada.

## Fuera de alcance
- Renombrar playlists en Navidrome.
- Cambiar el comportamiento de `AMAZON.FallbackIntent` ni de la paginación.
- Publicar o certificar la skill.
