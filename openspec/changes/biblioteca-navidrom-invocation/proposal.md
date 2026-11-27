# Cambio: invocación «biblioteca navidrom»

## Intención
Evitar que Alexa interprete «biblioteca musical» como la biblioteca de Amazon Music. Cambiar el invocation name del skill es-ES a `biblioteca navidrom` (dos palabras).

## Alcance
- Modelo de interacción es-ES (`interaction_model.json`).
- Tests de validación del modelo.
- Documentación y frases de prueba.

## Impacto
- No cambia el contrato HTTP, los intents existentes ni `sessionAttributes.listing`.
- Requiere re-importar el modelo y ejecutar Build Model en la consola Alexa.
- No cambia la imagen desplegada (`v0.2.0`).

## Fuera de alcance
- Añadir aliases: Alexa no los soporta.
- Publicar o certificar el skill.
