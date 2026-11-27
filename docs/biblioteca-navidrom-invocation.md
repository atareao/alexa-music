# biblioteca navidrom — cambio de invocación Alexa

## Qué es
Cambio del nombre de invocación del skill es-ES de `biblioteca musical` a `biblioteca navidrom` para evitar la colisión con la biblioteca de Amazon Music al probar en un Echo Dot.

## Ficheros implicados
- `interaction_model.json`: campo `interactionModel.languageModel.invocationName`.
- `tests/webhook_test.rs`: test `model_declares_biblioteca_navidrom_invocation_name`.
- `docs/alexa-signature-playlists.md`: frases de prueba.
- `HANDOFF.md`: estado y verificación pendiente.

## Cómo funciona
Alexa usa `invocationName` para enrutar la petición al skill. Con el nuevo valor, la frase de entrada es «Alexa, abre biblioteca navidrom» o «Alexa, abre biblioteca navidrom y dime qué álbumes tengo». Los intents y el cursor `sessionAttributes.listing = {"kind": ..., "offset": N}` no cambian.

## Variables/config
No hay variables de entorno nuevas. El endpoint HTTPS y `ALEXA_SKILL_ID` se mantienen.

## Cómo habilitar/desplegar
1. Re-importar `interaction_model.json` en Alexa Developer Console.
2. Ejecutar **Build Model**.
3. En simulador y Echo: «Alexa, abre biblioteca navidrom y dime qué álbumes tengo» → «dime más».
4. No requiere redesplegar `alexabridge`: la imagen `v0.2.0` no cambia.

## Decisiones
- Se descarta `navidrom` (una sola palabra): Alexa lo prohíbe salvo marca propia con prueba documental.
- Se descarta mantener aliases: Alexa no los soporta.
- Se elige `biblioteca navidrom` (dos palabras, minúsculas, sin artículos ni preposiciones).

## Follow-ups
- Confirmación en Echo físico con la misma cuenta Amazon.
- Tras el merge a `development`, cerrar `PLAN-004` y archivar el change OpenSpec.
