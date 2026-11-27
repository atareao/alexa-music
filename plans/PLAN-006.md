# PLAN-006 — Cambiar invocación a «servidor personal»

Issue: #42 — https://github.com/atareao/alexa-music/issues/42
Rama: `feature/servidor-personal` (desde `development`)
Supersede: PR #41 / issue #40 (cerrar sin merge)
Change OpenSpec: `openspec/changes/servidor-personal-invocation/`

## Objetivo
Sustituir `reproductor navidrom` por `servidor personal`. Tras dos renombres, el Echo sigue interpretando la frase como biblioteca de Amazon; se detectó además que la prueba de la skill podría estar en Off. Se adopta un nombre neutro, sin «biblioteca» ni «reproductor», que no coincida con funcionalidades integradas de Alexa.

## Mapa arquitectónico
- `interaction_model.json`: `interactionModel.languageModel.invocationName` pasa a `servidor personal`.
- `tests/webhook_test.rs`: actualizar/crear el test a `model_declares_servidor_personal_invocation_name` y su assert.
- `docs/servidor-personal-invocation.md`: renombrar el doc anterior (git mv) y actualizar contenido.
- `docs/alexa-signature-playlists.md` y `HANDOFF.md`: nuevas frases y estado del plan.
- PR #41 se cierra sin merge; se abre PR nuevo desde `feature/servidor-personal`.
- Sin cambios en `src/`, endpoints, intents ni `sessionAttributes.listing`.

## Tareas (TDD)
- [x] 1. RED: actualizar el test para exigir `invocationName == "servidor personal"`; `cargo test` debe fallar solo ahí.
- [x] 2. GREEN: cambiar `invocationName` en `interaction_model.json`; `cargo test` y `cargo check` en verde.
- [x] 3. REFACTOR: `cargo clippy --all-targets -- -D warnings` y `cargo fmt -- --check`.
- [x] 4. Docs: renombrar/actualizar `docs/servidor-personal-invocation.md`, ajustar `docs/alexa-signature-playlists.md` y `HANDOFF.md`.
- [x] 5. Validar JSON y reglas Alexa (dos palabras, minúsculas, sin artículos/preposiciones, sin «biblioteca» ni «reproductor»).
- [ ] 6. Verificación externa: **activar Test → Development en la consola** y habilitar la skill en *Alexa → Skills y juegos → Tus habilidades → En desarrollo*; re-importar modelo, Build Model y probar «Alexa, abre servidor personal y dime qué álbumes tengo» → «dime más» en simulador y Echo; confirmar `{"kind":...,"offset":N}` y revisar `docker logs --tail 100 alexabridge` por `dispatching Alexa request` / `401`.
- [ ] 7. Cierre: PR a `development`, cerrar PR #41, marcar tareas y renombrar a `plans/PLAN-006-ARCHIVED.md`; archivar el change; cerrar issues #38, #40 y #42.

## Comandos de verificación
`cargo test` · `cargo check` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt -- --check`

## Criterios de aceptación
- `cargo test` en verde con el nuevo nombre.
- `interaction_model.json` válido con `invocationName = servidor personal`.
- Con la prueba en **Development** y la habilidad habilitada: «Alexa, abre servidor personal» entra en la skill y no responde nada de biblioteca de Amazon.
