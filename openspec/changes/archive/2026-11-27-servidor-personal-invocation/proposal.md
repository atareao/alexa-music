# Cambio: invocación «servidor personal»

## Why
Sustituir `reproductor navidrom` por `servidor personal`. El Echo sigue interpretando la frase como biblioteca de Amazon; además, la prueba de la skill podría estar en Off. Se elige un nombre neutro sin «biblioteca» ni «reproductor».

## What Changes
- Modelo de interacción es-ES (`interaction_model.json`).
- Test de validación del modelo.
- Documentación y frases de prueba.
- Re-importar el modelo y Build Model; sustituir el PR #41.
- Verificar que *Test → Skill testing is enabled in* está en **Development** y que la skill aparece habilitada en la app Alexa.

## Impact
- No cambia el contrato HTTP, los intents ni `sessionAttributes.listing = {"kind", "offset"}`.
- No cambia la imagen desplegada (`v0.2.0`).

## Out of scope
- Publicar o certificar el skill.
- Aliases (Alexa no los soporta).
