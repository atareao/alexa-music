# reproductor navidrom — cambio de invocación Alexa

## Qué es
Cambio del nombre de invocación del skill es-ES a `reproductor navidrom` para evitar que Alexa interprete «biblioteca navidrom» como la biblioteca musical integrada durante el reconocimiento de voz.

## Evidencia
- El simulador en modo texto responde correctamente al modelo actual.
- El Echo Dot por voz responde «No encontré ninguna biblioteca Navidrome»; esa frase no existe en este repositorio.
- La skill está activa, el dispositivo está en es-ES y Build Model terminó sin errores.

## Ficheros implicados
- `interaction_model.json`: `interactionModel.languageModel.invocationName`.
- `tests/webhook_test.rs`: test `model_declares_reproductor_navidrom_invocation_name`.
- `docs/reproductor-navidrom-invocation.md`: este documento.
- `docs/alexa-signature-playlists.md`: frases de prueba.
- `HANDOFF.md`: estado y verificación pendiente.

## Cómo funciona
Alexa usa `invocationName` para enrutar la petición. Con el nuevo valor, se dice «Alexa, abre reproductor navidrom» o «Alexa, abre reproductor navidrom y dime qué álbumes tengo». Los intents y el cursor `sessionAttributes.listing = {"kind": ..., "offset": N}` no cambian.

## Variables/config
No hay variables de entorno nuevas. El endpoint HTTPS y `ALEXA_SKILL_ID` se mantienen.

## Cómo habilitar/desplegar
1. Re-importar `interaction_model.json` en Alexa Developer Console.
2. Ejecutar **Build Model**.
3. Probar en simulador y en Echo: «Alexa, abre reproductor navidrom y dime qué álbumes tengo» → «dime más».
4. No requiere redesplegar `alexabridge`: la imagen `v0.2.0` no cambia.

## Decisiones
- Se descarta `navidrom` (una sola palabra): Alexa lo prohíbe salvo marca propia con prueba documental.
- Se descarta `biblioteca navidrom`: colisión demostrada con la biblioteca musical integrada.
- Se elige `reproductor navidrom`: dos palabras, minúsculas, sin artículos ni preposiciones y sin la palabra problemática.

## Follow-ups
- Confirmación en Echo físico con la misma cuenta Amazon.
- Cerrar issues #38 y #40, cerrar PR #39 (ya cerrado) y archivar `PLAN-005` y el change OpenSpec tras la verificación.
