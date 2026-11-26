# Despliegue en co1 (Docker + Dockge)

## Qué es
El puente Alexa↔Navidrome se despliega como un servicio más del stack `navidrome` en `co1`, usando la imagen publicada en GHCR.

## Ficheros implicados (en co1)
- `/home/lorenzo/docker/dockge/stacks/navidrome/compose.yaml` — servicio `alexabridge` añadido.
- `/home/lorenzo/docker/dockge/stacks/navidrome/.env` — `ALEXA_FQDN`, `NAVIDROME_*`, `RUST_LOG`.
- Backups: `compose.yaml.bak-pre-alexa`, `.env.bak-pre-alexa`.

## Cómo funciona
- Traefik: `Host(alexa.territoriolinux.es)` → entrypoint `https`, certResolver `myresolver`, red `proxy`, puerto interno 3000.
- Navidrome: `https://musica.territoriolinux.es` (Subsonic v1.16.1).
- `SKIP_SIGNATURE_VERIFICATION=true` para pruebas con curl; poner `false` y configurar `ALEXA_SKILL_ID` cuando se importe el skill.

## Configuración
| Variable | Valor |
|---|---|
| FQDN webhook | `alexa.territoriolinux.es` |
| Imagen | `ghcr.io/atareao/alexa-music:v0.1.0` |
| NAVIDROME_URL | `https://musica.territoriolinux.es` |
| SKIP_SIGNATURE_VERIFICATION | `true` (temporal) |

## Cómo desplegar / reprobar
```bash
ssh co1
cd /home/lorenzo/docker/dockge/stacks/navidrome
docker compose up -d
curl -s https://alexa.territoriolinux.es/health
```
Payloads de prueba en `tests/`; el endpoint requiere headers `SignatureCertChainUrl` y `Signature` (falsos vales con SKIP=true).

## Decisiones y follow-ups
- Integrado en el stack de Navidrome (no stack separado) para compartir red y `.env`.
- `ALEXA_FQDN` separado de `FQDN` para no pisar el router de Navidrome.
- Follow-up: activar verificación RSA real y `ALEXA_SKILL_ID`; importar `interaction_model.json` en Alexa Developer Console con endpoint `https://alexa.territoriolinux.es/webhook`.
