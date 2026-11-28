# PLAN-004 — Cambiar invocación de skill a "biblioteca musical"

Issue: #50 — https://github.com/atareao/alexa-music/issues/50  
Rama: `feature/invocacion-biblioteca-musical` (desde `development`)  

## Objetivo
Cambiar el nombre de invocación del skill de `servidor personal` a `biblioteca musical` para evitar confusiones con funcionalidades integradas de Alexa.

## Mapa arquitectónico
- `interaction_model.json`: `invocationName` → `biblioteca musical`
- `tests/webhook_test.rs`: validación del nombre de invocación
- `HANDOFF.md`: actualización de documentación  
- `docs/servidor-personal-invocation.md`: actualización de la guía de cambio

## Tareas (TDD)

- [ ] 1. RED: añadir test que exija que `biblioteca musical == "biblioteca musical"`; `cargo test` debe fallar solo en ese caso.

- [ ] 2. GREEN: actualizar `interaction_model.json` con `invocationName: "biblioteca musical"`; `cargo test` y `cargo check` en verde.

- [ ] 3. REFACTOR: `cargo clippy --all-targets -- -D warnings` y `cargo fmt -- --check`.

- [ ] 4. Docs: crear/actualizar `docs/servidor-personal-invocation.md` y `HANDOFF.md` con evidencia de cambio y pasos de verificación.

- [ ] 5. Verificación externa: en la app Alexa (misma cuenta del Dot) → *Más → Skills y juegos → Tus habilidades → En desarrollo* → **Habilitar** `biblioteca musical`; en la consola Alexa, *Test* → *Development*; hacer *Build Model* y probar «Alexa, abre biblioteca musical y dime qué álbumes tengo» → «dime más».

- [ ] 6. Cierre: PR a `development`, marcar tareas, renombrar a `plans/PLAN-004-ARCHIVED.md`, archivar el change y cerrar el issue #50.

## Comandos de verificación
`cargo test` · `cargo check` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt -- --check` · `docker compose logs --tail 100 alexabridge` (verificar despliegue en co1)

## Criterios de aceptación
- `invocationName == "biblioteca musical"` en `interaction_model.json`
- Test `model_declares_servidor_personal_invocation_name` → `model_declares_biblioteca_musical_invocation_name` pasa
- Build Model en consola Alexa sin errores
- Habilitar skill en app Alexa → `biblioteca musical` disponible
- Probar en Echo: «Alexa, abre biblioteca musical y dime qué álbumes tengo» → `ListAlbumsIntent` → respuesta correcta
- Sin `401` ni errores de firma en `/webhook`
- Sin `no playlist matched` en búsquedas de playlists
- Sin regresiones en listados ni paginación
- 0 errores de firma en despliegue co1

## Nota
Este plan NO cierra `PLAN-006`: su tarea 6 sigue pendiente de confirmar «dime más» en el Dot (los `MoreListingIntent` registrados podrían venir del simulador). El rename a `biblioteca musical` está verificado: el Dot abre la skill sin `401`.
