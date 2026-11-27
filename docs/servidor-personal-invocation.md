# servidor personal — cambio de invocación Alexa

## Qué es
Cambio del nombre de invocación del skill es-ES a `servidor personal`, un nombre neutro que evita colisiones con «biblioteca» y «reproductor», ambos interpretados por Alexa como funcionalidades integradas.

## Evidencia
- El simulador en modo texto enruta correctamente al skill con los modelos anteriores.
- El Echo Dot por voz sigue respondiendo sobre la biblioteca de Amazon; esa frase no existe en este repositorio.
- La skill no aparecía en «Tus habilidades». La consola Alexa tiene el interruptor *Test → Skill testing is enabled in*, que debe estar en **Development** para que la skill esté disponible en la app y en el dispositivo.

## Ficheros implicados
- `interaction_model.json`: `interactionModel.languageModel.invocationName`.
- `tests/webhook_test.rs`: test `model_declares_servidor_personal_invocation_name`.
- `docs/servidor-personal-invocation.md`: este documento.
- `docs/alexa-signature-playlists.md`: frases de prueba.
- `HANDOFF.md`: estado y verificación pendiente.

## Cómo funciona
Alexa usa `invocationName` para enrutar la petición. Con el nuevo valor se dice «Alexa, abre servidor personal» o «Alexa, abre servidor personal y dime qué álbumes tengo». Los intents y el cursor `sessionAttributes.listing = {"kind": ..., "offset": N}` no cambian.

## Variables/config
No hay variables de entorno nuevas. El endpoint HTTPS y `ALEXA_SKILL_ID` se mantienen.

## Cómo habilitar/desplegar
1. En la consola Alexa, pestaña **Test**, poner *Skill testing is enabled in* = **Development** (ni Off ni Live). Si hay otra versión en Live, desactivarla.
2. En la app Alexa (misma cuenta del Dot): *Más → Skills y juegos → Tus habilidades → En desarrollo* → **Habilitar** `servidor personal`.
3. Re-importar `interaction_model.json` y ejecutar **Build Model**.
4. Probar en simulador y Echo: «Alexa, abre servidor personal y dime qué álbumes tengo» → «dime más».
5. Revisar en co1 `docker logs --tail 100 alexabridge`: debe aparecer `dispatching Alexa request` y no `401`.
6. No requiere redesplegar `alexabridge`: la imagen `v0.2.0` no cambia.

## Decisiones
- Se descarta una sola palabra: Alexa lo prohíbe salvo marca propia con prueba documental.
- Se descartan `biblioteca navidrom` y `reproductor navidrom`: colisiones demostradas o de enrutamiento.
- Se elige `servidor personal`: dos palabras, minúsculas, sin artículos ni preposiciones.

## Follow-ups
- Confirmación en Echo físico con la skill habilitada en Development.
- Cerrar issues #38, #40 y #42; archivar `PLAN-006` y el change OpenSpec tras la verificación.
