# Firma Alexa, Audio Player, playlists e intents de listado

## Qué es
Documentación de la feature que dejó el puente Alexa↔Navidrome operativo de extremo a extremo: verificación real de la firma de Alexa, reproducción mediante `AudioPlayer.Play`, limpieza de consultas habladas, lectura correcta de las canciones de una playlist de Navidrome OpenSubsonic y, desde `v0.2.0`, listados paginados de álbumes, artistas y playlists.

Estado: release/tag `v0.2.0` e imagen `ghcr.io/atareao/alexa-music:v0.2.0` publicados. En co1 sigue desplegada `v0.1.9` (healthy) hasta aplicar el despliegue pendiente de `v0.2.0`.

## Ficheros implicados
- `src/middleware.rs` — validación de headers y verificación RSA sobre el body crudo; `Signature-256` preferida, `Signature` legacy SHA-1 como compatibilidad; caché de certificados.
- `src/app.rs` — middleware global (`/health` exento), captura del body sin modificarlo, rechazo 401 y comprobación del ID del skill.
- `src/alexa.rs` — despacho de intents, generación de directivas `AudioPlayer.Play` sin `shouldEndSession`, `sanitize_search_query` y manejo de `PlayPlaylistIntent`.
- `src/subsonic.rs` — `get_playlist` lee `entry` (OpenSubsonic) y `song` (compatibilidad), con dedup por id; añade `get_album_list(size, offset)` y `get_artists()`.
- `interaction_model.json` — modelo es-ES: samples flexibles para `PlayPlaylistIntent`, sin `samples` en intents `AMAZON.*` y samples para los cuatro intents de listado de `v0.2.0`.
- `tests/webhook_test.rs` — cobertura de los intents de listado, del cursor de paginación y del flujo `MoreListingIntent`.
- `PLAN.md` — plan general actualizado con el alcance de los listados.
- `plans/PLAN-003-ARCHIVED.md` — plan de la feature de intents de listado (issue #31).
- `openspec/changes/archive/2026-11-27-list-library-intents/` — propuesta y deltas de spec de los intents de listado.
- `compose.yaml` en co1 — servicio `alexabridge`; imagen `v0.1.9` desplegada, pendiente pasar a `v0.2.0` (`/home/lorenzo/docker/dockge/stacks/navidrome/compose.yaml`).

## Cómo funciona
1. Alexa envía el request a `https://alexa.territoriolinux.es/webhook` con `SignatureCertChainUrl` y `Signature-256` (o `Signature` legacy).
2. El middleware valida la URL del certificado y decodifica la firma; `app.rs` verifica la firma criptográfica contra el **body crudo**, no contra un body re-serializado. Sin headers válidos la respuesta es 401.
3. Si `ALEXA_SKILL_ID` está configurado, se rechazan requests de otros skills.
4. Para `PlayMusicIntent`, `sanitize_search_query` elimina prefijos (`musica de`, `música de`, `pon`, `reproduce`) antes de llamar a `search3` de Navidrome.
5. Para `PlayPlaylistIntent`, se buscan las playlists, se resuelve la elegida y se obtienen sus canciones desde el campo `entry` de OpenSubsonic (o `song` por compatibilidad), deduplicando por id.
6. La respuesta incluye `AudioPlayer.Play` **sin `shouldEndSession`**, para que el dispositivo mantenga la sesión de audio y encadene las canciones. Los eventos `AudioPlayer.PlaybackStarted`, `PlaybackNearlyFinished`, etc. actualizan la cola en memoria por `deviceId`.

## Intents de listado de biblioteca (v0.2.0)

### Qué son
Cuatro intents nuevos permiten consultar el contenido de la biblioteca de Navidrome por voz y continuar un listado en turnos sucesivos:

- `ListAlbumsIntent` — enumera álbumes en orden alfabético por nombre.
- `ListArtistsIntent` — enumera artistas.
- `ListPlaylistsIntent` — enumera playlists.
- `MoreListingIntent` — continúa el último listado iniciado ("dime más").

### Ficheros implicados
- `src/alexa.rs` — despacho de los cuatro intents, construcción de la respuesta y lectura/escritura de `sessionAttributes.listing`.
- `src/subsonic.rs` — `get_album_list(size, offset)` y `get_artists()`; la consulta de playlists reutiliza la resolución existente.
- `interaction_model.json` — samples de los cuatro intents para locale es-ES.
- `tests/webhook_test.rs` — pruebas de los intents, del cursor de paginación y del flujo "dime más".
- `PLAN.md`, `plans/PLAN-003-ARCHIVED.md` y `openspec/changes/archive/2026-11-27-list-library-intents/` — plan y especificación del cambio (issue #31).

### Cómo funcionan
1. Alexa clasifica la frase y envía el intent al webhook con la firma verificada (mismo flujo de seguridad descrito arriba).
2. `ListAlbumsIntent` llama a `get_album_list(size, offset)`, que consulta `getAlbumList2?type=alphabeticalByName` en Navidrome.
3. `ListArtistsIntent` llama a `get_artists()`, que consulta `getArtists` y aplana `artists.index[].artist[]` en una lista plana de artistas.
4. `ListPlaylistsIntent` reutiliza la búsqueda de playlists ya presente en el puente.
5. Cada respuesta devuelve una página de 10 elementos y guarda en la sesión un cursor mínimo para poder continuar.
6. `MoreListingIntent` recupera ese cursor, vuelve a leer Navidrome con el `offset` acumulado y devuelve la página siguiente; si no hay listado previo, responde de forma coherente sin inventar datos.

### Ejemplos de voz
| Frase | Intent |
|---|---|
| «Alexa, abre biblioteca navidrom y dime qué álbumes tengo» | `ListAlbumsIntent` (invocación + listado) |
| «qué álbumes tengo» | `ListAlbumsIntent` |
| «qué artistas tengo» | `ListArtistsIntent` |
| «qué playlists tengo» | `ListPlaylistsIntent` |
| «dime más» | `MoreListingIntent` |

### Paginación
- Tamaño de página: **10 elementos**.
- Cursor: `sessionAttributes.listing = {"kind": "...", "offset": N}`, donde `kind` identifica qué listado se está recorriendo y `offset` es el número de elementos ya devueltos.
- El cursor guarda **solo** `kind` y `offset`; no se almacenan los nombres ni objetos ya leídos. Esto evita superar el límite de ~24 KB de `sessionAttributes` de Alexa.
- En "dime más" se reconsulta Navidrome con ese `offset`. Para álbumes, `get_album_list(size, offset)` se traduce en una petición `getAlbumList2?type=alphabeticalByName&size=offset+10&offset=0`, de la que se toman los elementos a partir de `offset`.

### Configuración
- No hay variables de entorno nuevas: se mantienen `NAVIDROME_URL`, `NAVIDROME_USER`, `NAVIDROME_PASSWORD`, `ALEXA_SKILL_ID`, `ALEXA_FQDN`, `SKIP_SIGNATURE_VERIFICATION=false` y `RUST_LOG`.
- En la consola de Alexa hay que importar `interaction_model.json` y ejecutar **Build Model** para publicar los samples de los cuatro intents; sin ese paso el simulator no los reconoce. No añadir `supportedInterfaces` al JSON.

### Despliegue
```bash
# Tras el tag v0.2.0:
cargo test
cargo check
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check

# En co1 (pendiente):
ssh co1
cd /home/lorenzo/docker/dockge/stacks/navidrome
docker compose pull alexabridge
docker compose up -d alexabridge
docker compose ps alexabridge
```
Después, en la consola Alexa: *Build Model* y prueba con las cuatro frases de ejemplo y un "dime más" encadenado.

### Decisiones y follow-ups
- Se pagina de 10 en 10 para que las respuestas sean digeribles por voz y el estado se mantenga pequeño.
- El cursor mínimo (`kind` + `offset`) es deliberado: guardar los resultados en `sessionAttributes` puede rebasar el límite de ~24 KB de Alexa.
- Los álbumes se ordenan por `alphabeticalByName` y los artistas se aplanan desde `artists.index[].artist[]` para normalizar la respuesta.
- Follow-ups: desplegar `v0.2.0` en co1, ejecutar *Build Model* en la consola, validar las frases y el "dime más" en el simulador y, finalmente, confirmar la audición en un Echo físico.

## Cambios por versión
| Versión | Corrección |
|---|---|
| `v0.1.1` | Verificación RSA sobre el body crudo, con `Signature-256` preferida y `Signature` legacy SHA-1. |
| `v0.1.2` | `AudioPlayer.Play` sin `shouldEndSession`. |
| `v0.1.5` | `sanitize_search_query` elimina prefijos antes de `search3`. |
| `v0.1.7` | Misma sanitización aplicada a `PlayPlaylistIntent`. |
| `v0.1.9` | `get_playlist` lee `entry` (y `song` por compatibilidad) con dedup por id. |
| `v0.2.0` | Intents `ListAlbumsIntent`, `ListArtistsIntent`, `ListPlaylistsIntent` y `MoreListingIntent`; paginación de 10 con cursor `sessionAttributes.listing`; `get_album_list` y `get_artists`. |

## Variables y configuración
| Variable | Uso |
|---|---|
| `SKIP_SIGNATURE_VERIFICATION` | Debe ser `false` en producción; `true` solo para desarrollo local con curl. |
| `ALEXA_SKILL_ID` | ID del skill permitido; configurado en el entorno de co1. |
| `ALEXA_FQDN` | FQDN público del webhook (`alexa.territoriolinux.es`). |
| `NAVIDROME_URL`, `NAVIDROME_USER`, `NAVIDROME_PASSWORD` | Conexión Subsonic a Navidrome; secretos solo en el `.env` del stack, jamás en docs ni en el repo. |
| `RUST_LOG` | Nivel de logging para diagnosticar `dispatching Alexa request`, `loading playlist tracks` y `playback started`. |

Consola de Alexa:
- Skill Custom, locale es-ES, invocation name `biblioteca navidrom`.
- Endpoint `https://alexa.territoriolinux.es/webhook`, certificado de CA pública (Let's Encrypt), sin wildcard.
- Interfaz **Audio Player** activada manualmente en *Build → Interfaces* y modelo recompilado con *Build Model*.
- Con `v0.2.0`, volver a ejecutar **Build Model** para publicar los samples de `ListAlbumsIntent`, `ListArtistsIntent`, `ListPlaylistsIntent` y `MoreListingIntent`.
- **No** añadir `supportedInterfaces` a `interaction_model.json`: la consola lo elimina al guardar.

## Cómo habilitar / desplegar
```bash
# En el repo, tras el tag v0.2.0:
cargo test
cargo check
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check

# En co1:
ssh co1
cd /home/lorenzo/docker/dockge/stacks/navidrome
docker compose pull alexabridge
docker compose up -d alexabridge
docker compose ps alexabridge
```

Verificación rápida (sin credenciales, sin generar firmas):
```bash
curl -i https://alexa.territoriolinux.es/health
curl -i -X POST https://alexa.territoriolinux.es/webhook \
  -H 'Content-Type: application/json' -d '{}'   # esperado: 401
docker logs --tail 100 alexabridge                # en co1
```

## Verificación final
- `/health` → HTTP 200 `{"status":"ok"}`.
- POST a `/webhook` sin headers → HTTP 401.
- Logs: `dispatching Alexa request intent=Some("PlayPlaylistIntent")`, `loading playlist tracks playlist=clasica song_count=Some(11)`, `starting playback track_id=6tat8goav82QFNlVTIs7ng`, `playback started token=track-6tat8goav82QFNlVTIs7ng error=None`.
- Simulador web: `Reproduciendo la lista clasica.`
- 101 tests verdes con `v0.2.0`; `cargo check`, `cargo clippy --all-targets -- -D warnings` y `cargo fmt -- --check` limpios; auditoría ✅ APROBADA.
- `navidrome` sigue Up y no fue reiniciado.
- La verificación de firma/audio listada arriba corresponde a `v0.1.9`, desplegada en co1; `v0.2.0` está publicada pero **pendiente de despliegue** y de *Build Model*.

## Decisiones
- La firma se verifica sobre el body crudo para evitar diferencias de serialización JSON entre Alexa y Axum.
- `Signature-256` manda cuando ambas firmas están presentes; `Signature` SHA-1 queda solo como fallback legado.
- `shouldEndSession` se omite en `AudioPlayer.Play` para no cortar la sesión de audio.
- La sanitización se aplica en ambos intents de reproducción para que las palabras de activación no contaminen la búsqueda.
- OpenSubsonic cambió `song` por `entry`; se aceptan ambos y se deduplica por id para evitar canciones repetidas.
- La interfaz Audio Player se activa en la consola porque el modelo declarado en JSON no es persistente allí.
- El cursor de los listados guarda solo `kind` y `offset` para no superar el límite de ~24 KB de `sessionAttributes` de Alexa.

## Seguimiento
- Issue: [#16](https://github.com/atareao/alexa-music/issues/16). PRs: #17/#18 (firma), #19/#20 (modelo AudioPlayer), #21/#22 (sanitize Query), #23/#24 (samples AMAZON), #25/#26 (sanitize playlist), #27/#28 (entry playlist).
- Listados `v0.2.0`: issue [#31](https://github.com/atareao/alexa-music/issues/31), PRs #32/#33; detalle en `plans/PLAN-003-ARCHIVED.md` y `openspec/changes/archive/2026-11-27-list-library-intents/`.
- Documentación operativa: `docs/deploy-co1.md`.
- **Pendiente:** desplegar `v0.2.0` en co1 (`docker compose pull/up -d alexabridge`), ejecutar *Build Model* en la consola Alexa si aún no se ha hecho y probar «Alexa, abre biblioteca navidrom y dime qué álbumes tengo» / «qué álbumes tengo» / «qué artistas tengo» / «qué playlists tengo» / «dime más». Sigue pendiente la confirmación auditiva en un Echo físico vinculado a la misma cuenta.
