# PLAN-001 — CI/CD: publicación de versiones en GitHub Container Registry

## Issue
Ref: https://github.com/atareao/alexa-music/issues/1

## Objetivo
Implementar un pipeline de CI/CD que verifique el código y publique versiones del microservicio Alexa↔Navidrome como imágenes OCI en `ghcr.io/atareao/alexa-music`, siguiendo el flujo gitflow del repositorio (feature → PR a development → PR a main).

## Arquitectura
- **CI (verificación):** `.github/workflows/ci.yml` se ejecuta en `pull_request` y en `push` a `main`/`development`. Pasos: `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`. Sin permisos de escritura en packages.
- **CD (publicación):** `.github/workflows/release.yml` se ejecuta en `push` a `main`/`development` y en `push` de tags `v*`. Construye desde `Containerfile` con `docker/build-push-action` + `docker/setup-buildx-action`, hace login en `ghcr.io` con `GITHUB_TOKEN` (permiso `packages: write`), y publica tags semánticos con `docker/metadata-action`.
- **Tags:** `latest` (solo en tag `v*` y `main`), `main`, `development`, `vX.Y.Z`, `X.Y.Z`, `sha-<corto>`.
- **Seguridad:** los PRs nunca publican; el token es el provisto por GitHub Actions; no se almacenan secretos adicionales en el workflow.

## Tareas

### Tarea 1: Bootstrap Git y gitflow
- [ ] Paso 1: Crear `.gitignore` con `target/`, `.env`, `*.env`, secrets.
- [ ] Paso 2: Commit inicial en `main` con todo el código existente (mensaje gitmoji: `🔧 chore: initial import of Alexa-Navidrome bridge`).
- [ ] Paso 3: Crear rama `development` desde `main` y pushear ambas.
- [ ] Paso 4: Crear rama `feature/cicd` desde `development`.

### Tarea 2: Workflow de CI
- [ ] Paso 1: Crear `.github/workflows/ci.yml` con trigger `pull_request` y `push` a `main`/`development`.
- [ ] Paso 2: Job `verify` en `ubuntu-latest`: checkout, `dtolnay/rust-toolchain@stable` con componentes `rustfmt`, `clippy`; cache de Cargo (Swatinem/rust-cache); `cargo fmt -- --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test`.
- [ ] Paso 3: `permissions: contents: read` (sin packages: write).

### Tarea 3: Workflow de CD
- [ ] Paso 1: Crear `.github/workflows/release.yml` con trigger `push` a `main`, `development` y tags `v*`.
- [ ] Paso 2: Job `publish`: `permissions: contents: read, packages: write`; checkout; `docker/setup-qemu-action` y `docker/setup-buildx-action`; login a `ghcr.io` con `docker/login-action` usuario `${{ github.actor }}`, password `${{ secrets.GITHUB_TOKEN }}`.
- [ ] Paso 3: `docker/metadata-action` con imágenes `ghcr.io/atareao/alexa-music` y tags: `type=raw,value=latest,enable={{is_default_branch}}`, `type=ref,event=branch`, `type=semver,pattern={{version}}`, `type=semver,pattern=v{{version}}`, `type=sha`.
- [ ] Paso 4: `docker/build-push-action` con `context: .`, `file: ./Containerfile`, `push: true`, `tags`/`labels` del metadata-action, `cache-from`/`cache-to` GHA.
- [ ] Paso 5: Añadir comentario indicando que la imagen se consume en `alexabridge.container` / `compose.yml` como `ghcr.io/atareao/alexa-music:latest`.

### Tarea 4: Documentación
- [ ] Paso 1: Crear `docs/cicd.md`: qué es, ficheros implicados, cómo funciona, variables/permisos, cómo publicar una versión (git tag vX.Y.Z + push), cómo probar localmente, decisiones y follow-ups.
- [ ] Paso 2: Actualizar `HANDOFF.md` con el nuevo estado (CI/CD implementado) y el flujo de release.

### Tarea 5: Verificación y cierre
- [ ] Paso 1: Validar YAML de ambos workflows (`actionlint` si está disponible, o parser YAML).
- [ ] Paso 2: Confirmar que PRs no tienen permisos de publicación.
- [ ] Paso 3: PR `feature/cicd` → `development`; tras aprobación, PR `development` → `main`.
- [ ] Paso 4: Verificar en GitHub Actions que el job de CI pasa y, tras merge a main, que la imagen aparece en ghcr.io con tags.

## Criterios de aceptación
- Workflows sintácticamente válidos.
- CI verde en PR.
- Imagen publicada en ghcr.io con tags semánticos tras merge a main/tag.
- Docs y HANDOFF actualizados.
