# Firma Alexa, Audio Player y reproducción de playlists

## Qué es
Documentación de la feature que dejó el puente Alexa↔Navidrome operativo de extremo a extremo: verificación real de la firma de Alexa, reproducción mediante `AudioPlayer.Play`, limpieza de consultas habladas y lectura correcta de las canciones de una playlist de Navidrome OpenSubsonic.

Estado final: release/tag `v0.1.9`, imagen `ghcr.io/atareao/alexa-music:v0.1.9` desplegada y healthy en co1.

## Ficheros implicados
- `src/middleware.rs` — validación de headers y verificación RSA sobre el body crudo; `Signature-256` preferida, `Signature` legacy SHA-1 como compatibilidad; caché de certificados.
- `src/app.rs` — middleware global (`/health` exento), captura del body sin modificarlo, rechazo 401 y comprobación del ID del skill.
- `src/alexa.rs` — despacho de intents, generación de directivas `AudioPlayer.Play` sin `shouldEndSession`, `sanitize_search_query` y manejo de `PlayPlaylistIntent`.
- `src/subsonic.rs` — `get_playlist` lee `entry` (OpenSubsonic) y `song` (compatibilidad), con dedup por id.
- `interaction_model.json` — modelo es-ES: samples flexibles para `PlayPlaylistIntent` y sin `samples` en intents `AMAZON.*`.
- `compose.yaml` en co1 — servicio `alexabridge` con la imagen `v0.1.9` dentro del stack `navidrome` (`/home/lorenzo/docker/dockge/stacks/navidrome/compose.yaml`).

## Cómo funciona
1. Alexa envía el request a `https://alexa.territoriolinux.es/webhook` con `SignatureCertChainUrl` y `Signature-256` (o `Signature` legacy).
2. El middleware valida la URL del certificado y decodifica la firma; `app.rs` verifica la firma criptográfica contra el **body crudo**, no contra un body re-serializado. Sin headers válidos la respuesta es 401.
3. Si `ALEXA_SKILL_ID` está configurado, se rechazan requests de otros skills.
4. Para `PlayMusicIntent`, `sanitize_search_query` elimina prefijos (`musica de`, `música de`, `pon`, `reproduce`) antes de llamar a `search3` de Navidrome.
5. Para `PlayPlaylistIntent`, se buscan las playlists, se resuelve la elegida y se obtienen sus canciones desde el campo `entry` de OpenSubsonic (o `song` por compatibilidad), deduplicando por id.
6. La respuesta incluye `AudioPlayer.Play` **sin `shouldEndSession`**, para que el dispositivo mantenga la sesión de audio y encadene las canciones. Los eventos `AudioPlayer.PlaybackStarted`, `PlaybackNearlyFinished`, etc. actualizan la cola en memoria por `deviceId`.

## Cambios por versión
| Versión | Corrección |
|---|---|
| `v0.1.1` | Verificación RSA sobre el body crudo, con `Signature-256` preferida y `Signature` legacy SHA-1. |
| `v0.1.2` | `AudioPlayer.Play` sin `shouldEndSession`. |
| `v0.1.5` | `sanitize_search_query` elimina prefijos antes de `search3`. |
| `v0.1.7` | Misma sanitización aplicada a `PlayPlaylistIntent`. |
| `v0.1.9` | `get_playlist` lee `entry` (y `song` por compatibilidad) con dedup por id. |

## Variables y configuración
| Variable | Uso |
|---|---|
| `SKIP_SIGNATURE_VERIFICATION` | Debe ser `false` en producción; `true` solo para desarrollo local con curl. |
| `ALEXA_SKILL_ID` | ID del skill permitido; configurado en el entorno de co1. |
| `ALEXA_FQDN` | FQDN público del webhook (`alexa.territoriolinux.es`). |
| `NAVIDROME_URL`, `NAVIDROME_USER`, `NAVIDROME_PASSWORD` | Conexión Subsonic a Navidrome; secretos solo en el `.env` del stack, jamás en docs ni en el repo. |
| `RUST_LOG` | Nivel de logging para diagnosticar `dispatching Alexa request`, `loading playlist tracks` y `playback started`. |

Consola de Alexa:
- Skill Custom, locale es-ES, invocation name `biblioteca musical`.
- Endpoint `https://alexa.territoriolinux.es/webhook`, certificado de CA pública (Let's Encrypt), sin wildcard.
- Interfaz **Audio Player** activada manualmente en *Build → Interfaces* y modelo recompilado con *Build Model*.
- **No** añadir `supportedInterfaces` a `interaction_model.json`: la consola lo elimina al guardar.

## Cómo habilitar / desplegar
```bash
# En el repo, tras el tag v0.1.9:
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
- 88 tests verdes; `cargo check`, `cargo clippy --all-targets -- -D warnings` y `cargo fmt -- --check` limpios en el último cambio.
- `navidrome` sigue Up y no fue reiniciado.

## Decisiones
- La firma se verifica sobre el body crudo para evitar diferencias de serialización JSON entre Alexa y Axum.
- `Signature-256` manda cuando ambas firmas están presentes; `Signature` SHA-1 queda solo como fallback legado.
- `shouldEndSession` se omite en `AudioPlayer.Play` para no cortar la sesión de audio.
- La sanitización se aplica en ambos intents de reproducción para que las palabras de activación no contaminen la búsqueda.
- OpenSubsonic cambió `song` por `entry`; se aceptan ambos y se deduplica por id para evitar canciones repetidas.
- La interfaz Audio Player se activa en la consola porque el modelo declarado en JSON no es persistente allí.

## Seguimiento
- Issue: [#16](https://github.com/atareao/alexa-music/issues/16). PRs: #17/#18 (firma), #19/#20 (modelo AudioPlayer), #21/#22 (sanitize Query), #23/#24 (samples AMAZON), #25/#26 (sanitize playlist), #27/#28 (entry playlist).
- Documentación operativa: `docs/deploy-co1.md`.
- **Pendiente único:** confirmar la audición en un Echo físico vinculado a la misma cuenta. El simulador web no reproduce audio; los logs confirman `AudioPlayer.PlaybackStarted`.
