# CI/CD — Publicación en GitHub Container Registry

## Qué es
Pipeline de GitHub Actions que verifica el código Rust y publica la imagen OCI del puente Alexa↔Navidrome en `ghcr.io/atareao/alexa-music`.

## Ficheros implicados
- `.github/workflows/ci.yml` — verificación en PRs y pushes a `main`/`development`.
- `.github/workflows/release.yml` — publicación en `ghcr.io`.
- `Containerfile` — definición multi-stage de la imagen.
- `compose.yml` / `alexabridge.container` — consumen la imagen publicada `ghcr.io/atareao/alexa-music:latest` (en `compose.yml`, `build:` permanece comentado; descomentarlo solo para builds locales).

## Cómo funciona
### CI (verificación)
- Triggers: `pull_request` y `push` a `main` / `development`.
- Pasos: `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- Permisos: solo `contents: read`. Los PRs **nunca** publican.

### CD (publicación)
- Triggers: `push` a `main`, `development` y tags `v*`.
- Construye desde `Containerfile` con Buildx, login en `ghcr.io` vía `GITHUB_TOKEN` (`packages: write`).
- Tags generados: `latest` (solo en `main`), `main`, `development`, `vX.Y.Z`, `X.Y.Z`, `sha-<corto>`.
- Concurrencia: un solo publish a la vez por rama/tag.

## Variables y permisos
| Elemento | Valor |
|---|---|
| Registry | `ghcr.io` |
| Imagen | `ghcr.io/atareao/alexa-music` |
| Token | `${{ secrets.GITHUB_TOKEN }}` (automático) |
| Permiso publicación | `packages: write` |
| Plataformas | `linux/amd64`, `linux/arm64` |

## Cómo publicar una versión
1. Desde `development`, crear release branch / verificar CI en PR.
2. Merge a `main` mediante PR.
3. Crear y pushear tag semver: `git tag v0.1.0 && git push origin v0.1.0`.
4. El workflow `release.yml` publica `ghcr.io/atareao/alexa-music:v0.1.0`, `:0.1.0` y `:sha-...`; `:latest` se publica únicamente en `main`.

## Cómo probar localmente
- `act -j verify` (si usas nektos/act) o ejecutar los comandos del CI en local.
- `docker build -f Containerfile -t alexa-music:test .` para validar la imagen.
- `docker compose config` para validar el compose.

## Decisiones y follow-ups
- Se usa `GITHUB_TOKEN` en lugar de PAT: el registry es el mismo repo, sin secretos extra.
- `latest` se reserva para `main` (branch por defecto) para no sobreescribir con development.
- Follow-up: añadir escaneo de vulnerabilidades (Trivy) y firma de imagen (cosign).
