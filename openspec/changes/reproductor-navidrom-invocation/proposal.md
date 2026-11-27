# Cambio: invocación «reproductor navidrom»

## Intención
Sustituir `biblioteca navidrom` por `reproductor navidrom`. El simulador en modo texto enruta bien, pero el Echo Dot por voz responde «No encontré ninguna biblioteca Navidrome»: la palabra «biblioteca» colisiona con la biblioteca musical integrada de Alexa.

## Alcance
- Modelo de interacción es-ES (`interaction_model.json`).
- Test de validación del modelo.
- Documentación y frases de prueba.
- Re-importar el modelo y Build Model; sustituir el PR #39.

## Impacto
- No cambia el contrato HTTP, los intents ni `sessionAttributes.listing = {"kind", "offset"}`.
- No cambia la imagen desplegada (`v0.2.0`).

## Fuera de alcance
- Publicar o certificar el skill.
- Aliases (Alexa no los soporta).
