# Despliegue en co1 (Docker + Dockge)

## Qué es
El puente Alexa↔Navidrome se despliega como un servicio más del stack `navidrome` en `co1`, usando la imagen publicada en GHCR. Estado final: **desplegado, healthy y verificado** con la imagen `v0.1.9`.

## Ficheros implicados (en co1)
- `/home/lorenzo/docker/dockge/stacks/navidrome/compose.yaml` — servicio `alexabridge`, en el mismo stack que `navidrome`.
- `/home/lorenzo/docker/dockge/stacks/navidrome/.env` — variables del stack (contiene secretos; nunca copiar su contenido a documentación ni al repo).
- Backups creados durante el despliegue: `compose.yaml.bak-pre-alexa`, `.env.bak-pre-alexa`.

En el repo: `compose.yml`, `Containerfile`, `interaction_model.json`, `src/`, `docs/alexa-signature-playlists.md`.

## Cómo funciona
- Traefik: `Host(alexa.territoriolinux.es)` → entrypoint `https`, certResolver `myresolver`, red `proxy`, puerto interno `3000` del contenedor.
- Navidrome: `https://musica.territoriolinux.es` (Subsonic / OpenSubsonic). El contenedor `navidrome` sigue Up y **no ha sido reiniciado** por el despliegue del puente.
- El webhook público es `https://alexa.territoriolinux.es/webhook`; el healthcheck público es `https://alexa.territoriolinux.es/health`.
- Verificación de firma activada: `SKIP_SIGNATURE_VERIFICATION=false` y `ALEXA_SKILL_ID` configurado. No hay modo de prueba con firma desactivada en producción.
- Skill Alexa Custom, locale es-ES, invocation name `biblioteca musical`, endpoint `/webhook` con certificado de CA pública (Let's Encrypt), sin wildcard.

## Configuración (solo nombres, sin valores secretos)
| Variable / parámetro | Estado en co1 |
|---|---|
| Imagen | `ghcr.io/atareao/alexa-music:v0.1.9` |
| FQDN del webhook | `alexa.territoriolinux.es` |
| `NAVIDROME_URL` | `https://musica.territoriolinux.es` |
| `ALEXA_FQDN` | FQDN del servicio `alexabridge` (independiente del `FQDN` de Navidrome) |
| `NAVIDROME_USER` / `NAVIDROME_PASSWORD` | Definidas en el `.env` del stack (fuera del repo) |
| `SKIP_SIGNATURE_VERIFICATION` | `false` |
| `ALEXA_SKILL_ID` | Configurado con el ID del skill desplegado |
| `RUST_LOG` | Configurado en el stack para los logs de verificación |

La consola de Alexa tiene activada la interfaz **Audio Player** en *Build → Interfaces*, y el modelo fue recompilado con *Build Model*. `supportedInterfaces` no se incluye en `interaction_model.json`: la consola lo elimina.

## Cómo desplegar
```bash
ssh co1
cd /home/lorenzo/docker/dockge/stacks/navidrome
docker compose pull alexabridge
docker compose up -d alexabridge
docker compose ps alexabridge
```
Para publicar una nueva versión: crear el tag `vX.Y.Z` en GitHub (publica la imagen en GHCR), actualizar la imagen del servicio `alexabridge` en `compose.yaml`, y aplicar con `docker compose up -d alexabridge`.

## Verificación (sin credenciales)
```bash
# 1) Health público → HTTP 200 {"status":"ok"}
curl -i https://alexa.territoriolinux.es/health

# 2) POST sin headers de firma → HTTP 401 (la firma real está activada)
curl -i -X POST https://alexa.territoriolinux.es/webhook \
  -H 'Content-Type: application/json' -d '{}'

# 3) Logs del contenedor (en co1)
docker logs --tail 100 alexabridge
```

Resultado esperado en los logs tras una petición válida del skill:
- `dispatching Alexa request intent=Some("PlayPlaylistIntent")`
- `loading playlist tracks playlist=clasica song_count=Some(11)`
- `starting playback track_id=...`
- `playback started token=track-... error=None`

Verificación final realizada en co1 con `v0.1.9`: `/health` HTTP 200 con `{"status":"ok"}`; POST sin headers HTTP 401; `PlayPlaylistIntent` con la lista `clasica` (11 canciones) → `AudioPlayer.Play` y `AudioPlayer.PlaybackStarted` sin error. En el simulador web se mostró `Reproduciendo la lista clasica.`

## Decisiones y follow-ups
- Integrado en el stack de Navidrome (no stack separado) para compartir red y `.env`.
- `ALEXA_FQDN` separado de `FQDN` para no pisar el router de Navidrome.
- Verificación RSA sobre el body crudo, con `Signature-256` (SHA-256) preferida y `Signature` legacy (SHA-1) por compatibilidad.
- **Único pendiente:** confirmación auditiva en un Echo físico con la misma cuenta. El simulador web no reproduce audio, aunque los logs confirman `AudioPlayer.PlaybackStarted`.
- Detalle de correcciones, modelo y PRs relacionados: `docs/alexa-signature-playlists.md`.
