# PLAN-002 — Despliegue de alexa-music en co1 (Docker + Dockge)

## Issue
Ref: https://github.com/atareao/alexa-music/issues/7

## Objetivo
Desplegar el puente Alexa↔Navidrome en el servidor `co1` (Contabo, Docker + Dockge) usando la imagen publicada en GHCR, y probar el webhook con curl.

## Hechos verificados del entorno
- SSH: `ssh co1` → usuario `lorenzo`, host `167.86.73.171`.
- Dockge stacks: `/home/lorenzo/docker/dockge/stacks/<stack>/compose.yaml`.
- Navidrome: contenedor `navidrome`, red `proxy`, FQDN `musica.territoriolinux.es`.
- Traefik: entrypoint `https`, certResolver `myresolver`, red externa `proxy`.
- FQDN elegido: `alexa.territoriolinux.es` → resuelve a `167.86.73.171`.
- `co1` puede hacer pull de `ghcr.io/atareao/alexa-music:v0.1.0` (manifest OK).
- Modo prueba: `SKIP_SIGNATURE_VERIFICATION=true` (firma real se validará después con skill Alexa).
- Credenciales Navidrome: las proporciona el usuario en el `.env` del stack.

## Arquitectura
```
Internet HTTPS (Traefik :443, myresolver)
   └─ Host(alexa.territoriolinux.es) → red proxy → alexabridge:3000
                                                      └─ Navidrome (musica.territoriolinux.es, Subsonic)
```

## Tareas

### Tarea 1: Crear stack en Dockge
- [x] Paso 1: Crear directorio `/home/lorenzo/docker/dockge/stacks/alexa-music/`.
- [x] Paso 2: Crear `compose.yaml` con:
  - `image: ghcr.io/atareao/alexa-music:v0.1.0`
  - `container_name: alexabridge`, `init: true`, `restart: unless-stopped`, red `proxy`.
  - `env_file: .env`
  - labels: `traefik.enable: true`, `traefik.http.routers.alexabridge.rule: Host(\`${FQDN}\`)`, `traefik.http.routers.alexabridge.entrypoints: https`, `traefik.http.routers.alexabridge.tls.certresolver: myresolver`, `traefik.http.services.alexabridge.loadbalancer.server.port: 3000`.
  - `x-dockge.urls: https://${FQDN}`.
- [x] Paso 3: Crear `.env.example` con FQDN, NAVIDROME_URL, NAVIDROME_USER, NAVIDROME_PASSWORD, PORT, RUST_LOG, SKIP_SIGNATURE_VERIFICATION.

### Tarea 2: Configurar credenciales
- [x] Paso 1: El usuario crea `.env` con las credenciales reales de Navidrome (fuera del repo; nunca commitear).

### Tarea 3: Desplegar
- [x] Paso 1: `docker compose -f /home/lorenzo/docker/dockge/stacks/alexa-music/compose.yaml --env-file .../ .env config` para validar.
- [x] Paso 2: `docker compose up -d` desde el directorio del stack.
- [x] Paso 3: Verificar `docker ps` y logs (`docker logs alexabridge`).

### Tarea 4: Pruebas
- [x] Paso 1: `curl -s https://alexa.territoriolinux.es/health` → `{"status":"ok"}`.
- [x] Paso 2: POST `/webhook` con headers falsos válidos y payload `LaunchRequest` → 200 con texto de bienvenida en español.
- [x] Paso 3: POST con headers ausentes → 401.
- [x] Paso 4: Probar un `PlayMusicIntent` con consulta real contra Navidrome y verificar directiva `AudioPlayer.Play` con URL `/rest/stream?...&format=mp3`.

### Tarea 5: Cierre
- [x] Paso 1: Documentar en `docs/deploy-co1.md` los datos del despliegue y cómo reprobar.
- [x] Paso 2: Marcar este plan completado y actualizar HANDOFF.

## Criterios de aceptación
- Contenedor `alexabridge` corriendo en co1.
- `/health` 200 y `/webhook` 200/401 según headers.
- Búsqueda real contra Navidrome devuelve stream MP3.

## Estado final
- Desplegado en co1 dentro del stack `navidrome` (Dockge), contenedor `alexabridge`, imagen `ghcr.io/atareao/alexa-music:v0.1.0`.
- Verificado: `/health` 200 por HTTPS, `/webhook` 401 sin headers, LaunchRequest 200 en español, PlayMusicIntent con búsqueda real → `AudioPlayer.Play` con stream `audio/mpeg` (ID3/MP3).
- FQDN: https://alexa.territoriolinux.es · Navidrome: https://musica.territoriolinux.es.
- Modo actual: `SKIP_SIGNATURE_VERIFICATION=true` (pendiente de activar firma real con skill Alexa).
