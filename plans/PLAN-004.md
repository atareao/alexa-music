# PLAN-004 — Cambiar invocación Alexa a «biblioteca navidrom»

Issue: #38 — https://github.com/atareao/alexa-music/issues/38
Rama: `feature/biblioteca-navidrom` (desde `development`)
Change OpenSpec: `openspec/changes/biblioteca-navidrom-invocation/`

## Objetivo
Sustituir el nombre de invocación del skill es-ES `biblioteca musical` por `biblioteca navidrom` para evitar la colisión con la biblioteca de Amazon Music detectada al probar en un Echo Dot («No entiendo eso»).

## Mapa arquitectónico
- `interaction_model.json`: `invocationName` es el único punto de definición del nombre en el repo.
- `tests/webhook_test.rs`: test de validación del modelo → nueva aserción del nombre.
- `docs/alexa-signature-playlists.md`: sección de intents y frases de prueba → «Alexa, abre biblioteca navidrom».
- `HANDOFF.md`: pendiente #1 y comandos de verificación con la nueva frase.
- Consola Alexa Developer: re-importar el modelo y ejecutar Build Model (acción manual, fuera del repo).
- Sin cambios en `src/`, endpoints, despliegue ni imagen `v0.2.0`.

## Tareas (TDD)
- [x] 1. RED: crear/actualizar el test que exige `invocationName == "biblioteca navidrom"`; ejecutar `cargo test` y confirmar que falla con el valor actual.
- [x] 2. GREEN: cambiar `invocationName` en `interaction_model.json`; ejecutar `cargo test` y `cargo check` hasta verde.
- [x] 3. REFACTOR: `cargo clippy --all-targets -- -D warnings` y `cargo fmt -- --check`; sin cambios de comportamiento.
- [x] 4. Docs: actualizar `docs/alexa-signature-playlists.md` y `HANDOFF.md` con la nueva frase de invocación.
- [x] 5. Validar que el JSON sigue siendo válido y que el nombre cumple las reglas de Alexa (dos palabras, minúsculas, sin artículos/preposiciones).
- [ ] 6. Verificación externa: re-importar modelo, Build Model, probar «Alexa, abre biblioteca navidrom y dime qué álbumes tengo» en simulador y Echo físico; confirmar `sessionAttributes.listing = {"kind":...,"offset":N}`.
- [ ] 7. Cierre: PR a `development` con commits gitmoji; al aprobar, marcar tareas y renombrar a `plans/PLAN-004-ARCHIVED.md`; crear `docs/biblioteca-navidrom-invocation.md`.

## Comandos de verificación
`cargo test` · `cargo check` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt -- --check`

## Criterios de aceptación
- `cargo test` en verde con el nuevo nombre de invocación.
- `interaction_model.json` válido con `invocationName = biblioteca navidrom`.
- Build Model correcto y entrada por voz verificada en Echo: «Alexa, abre biblioteca navidrom» entra en la skill.
