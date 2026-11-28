# random-music — intents de reproducción aleatoria

## Qué es
Feature que añade tres intents es-ES para reproducir música aleatoria desde Alexa:
- `PlayRandomIntent`: reproduce una canción aleatoria de toda la biblioteca. Ejemplo: «Alexa, abre biblioteca musical y reprocúcela algo al azar».
- `PlayRandomArtistIntent`: reproduce canciones del artista indicado en orden aleatorio. Ejemplo: «Alexa, abre biblioteca musical y reproduce algo aleatorio de [Artista]».
- `PlayRandomPlaylistIntent`: reproduce canciones de la playlist indicada en orden aleatorio. Ejemplo: «Alexa, abre biblioteca musical y reproduce la playlist [Playlist] al azar».

## Ficheros implicados
- `interaction_model.json`: definición de los tres intents y sus samples en es-ES.
- `src/subsonic.rs`: métodos `get_random_songs`, `get_artist_songs` y `get_playlist_songs`, sobre el cliente Subsonic/OpenSubsonic existente.
- `src/alexa.rs`: handlers `handle_play_random`, `handle_play_random_artist` y `handle_play_random_playlist`, con su despacho en `handle_request`.
- `tests/webhook_test.rs`: validación del modelo con los tres intents.
- `HANDOFF.md`: estado del release y verificación.

## Cómo funciona
- Los tres intents reutilizan la cola de reproducción en memoria existente (`start_queue`): no se introduce un mecanismo de playback nuevo.
- `sessionAttributes` no cambia: no se añade estado nuevo; solo se mantiene el contrato ya usado por el AudioPlayer.
- El matching de nombres hablados de artista y playlist usa `normalize_for_playlist_search` (minúsculas y sin diacritíticos), por lo que «clásica» casa con `clasica`.
- `PlayRandomIntent` llama a `get_random_songs` (Subsonic `getRandomSongs`); `PlayRandomArtistIntent` resuelve el artista y usa `get_artist_songs`; `PlayRandomPlaylistIntent` resuelve la playlist y usa `get_playlist_songs`, barajando el resultado antes de encolarlo.
- Errores de Subsonic o de matching devuelven un mensaje claro al usuario, sin panic.

## Variables/config
Sin variables nuevas. Se mantienen `SKIP_SIGNATURE_VERIFICATION=false` en producción (firma RSA activa), `ALEXA_SKILL_ID` y el invocation name `biblioteca musical`. El endpoint HTTPS y el certificado de CA pública no cambian.

## Cómo habilitar/desplegar
Ya desplegado en `v0.2.2` en co1 (imagen `ghcr.io/atareao/alexa-music:v0.2.2`, tag sobre el commit de PR #53): `alexabridge` Up (healthy).
1. Re-importar `interaction_model.json` en la consola Alexa.
2. Ejecutar **Build Model** para compilar los tres intents nuevos.
3. Probar las frases de ejemplo en el Echo (con *Skill testing* en **Development** y la skill habilitada en la app).

## Decisiones
- La aleatoria global usa `getRandomSongs` de Subsonic en lugar de implementar un muestreo propio.
- Para artista y playlist se obtienen sus canciones y se reproducen por la cola existente (`start_queue`), barajándolas en el handler.
- No se añade base de datos ni estado persistente nuevo.
- No se toca la verificación de firma RSA (preferencia `Signature-256`, fallback SHA-1, caché TTL 24h).
- `sessionAttributes` permanece sin cambios para no alterar los intents de listado existentes.

## Follow-ups
- Verificado HTTP el 28-11-2026: `/health` → 200 y `/webhook` sin firma → 401.
- Issue #52 y PLAN-005 cerrados y plan archivado (`plans/PLAN-005-ARCHIVED.md`).
- Auditar en Echo las tres frases de ejemplo para confirmación auditiva del audio (el simulador web no reproduce audio).
