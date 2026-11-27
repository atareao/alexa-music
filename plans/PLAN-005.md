# PLAN-005 — Cambiar invocación a «reproductor navidrom»

Issue: #40 — https://github.com/atareao/alexa-music/issues/40
Rama: `feature/reproductor-navidrom` (desde `development`)
Supersede: PR #39 / issue #38 (cerrar sin merge)
Change OpenSpec: `openspec/changes/reproductor-navidrom-invocation/`

## Objetivo
Sustituir `biblioteca navidrom` por `reproductor navidrom`. El simulador en modo texto ya enruta al skill, pero el Echo Dot por voz responde «No encontré ninguna biblioteca Navidrome»: la palabra «biblioteca» colisiona con la biblioteca musical integrada de Alexa.

## Mapa arquitectónico
- `interaction_model.json`: `interactionModel.languageModel.invocationName` pasa a `reproductor navidrom`.
- `tests/webhook_test.rs`: actualizar el test de invocación a `model_declares_reproductor_navidrom_invocation_name` y su assert.
- `docs/reproductor-navidrom-invocation.md`: renombrar el doc anterior (git mv) y actualizar su contenido.
- `docs/alexa-signature-playlists.md` y `HANDOFF.md`: nuevas frases de prueba y estado del plan.
- PR #39 se cierra sin merge; se abre PR nuevo desde `feature/reproductor-navidrom`.
- Sin cambios en `src/`, endpoints, intents ni `sessionAttributes.listing`.

## Tareas (TDD)
- [x] 1. RED: actualizar el test para exigir `invocationName == "reproductor navidrom"`; `cargo test` debe fallar solo ahí.
- [x] 2. GREEN: cambiar `invocationName` en `interaction_model.json`; `cargo test` y `cargo check` en verde.
- [x] 3. REFACTOR: `cargo clippy --all-targets -- -D warnings` y `cargo fmt -- --check`.
- [x] 4. Docs: renombrar/actualizar `docs/reproductor-navidrom-invocation.md`, ajustar `docs/alexa-signature-playlists.md` y `HANDOFF.md`.
- [x] 5. Validar JSON y reglas Alexa (dos palabras, minúsculas, sin artículos/preposiciones ni «biblioteca»).
- [ ] 6. Verificación externa: re-importar modelo, Build Model, probar «Alexa, abre reproductor navidrom y dime qué álbumes tengo» → «dime más» en simulador y Echo; confirmar `{"kind":...,"offset":N}`.
- [ ] 7. Cierre: PR a `development`, cerrar PR #39, marcar tareas y renombrar a `plans/PLAN-005-ARCHIVED.md`; archivar el change; cerrar issues #38 y #40.

## Comandos de verificación
`cargo test` · `cargo check` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt -- --check`

## Criterios de aceptación
- `cargo test` en verde con el nuevo nombre.
- `interaction_model.json` válido con `invocationName = reproductor navidrom`.
- Build Model correcto y entrada por voz verificada en Echo: «Alexa, abre reproductor navidrom» entra en la skill y no responde «No encontré ninguna biblioteca Navidrome».
