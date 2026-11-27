# Delta: `alexa-invocation-servidor`

## Contratos
- `interaction_model.json.interactionModel.languageModel.invocationName` (string): debe ser exactamente `servidor personal` para `es-ES`.
- Frases soportadas: `Alexa, abre servidor personal` y `Alexa, abre servidor personal y dime <petición>`.
- `sessionAttributes.listing` conserva su contrato: `{"kind": string, "offset": number}`; este cambio no lo modifica.

## Escenarios

### Escenario: el modelo declara el nuevo nombre
- Given el fichero `interaction_model.json`
- When se valida el modelo en la suite de tests
- Then el JSON es válido y `invocationName` es `servidor personal`.

### Escenario: prueba habilitada en Development
- Given la consola Alexa con el modelo reconstruido
- When se activa *Skill testing is enabled in = Development* y se habilita la skill en la app Alexa
- Then la skill aparece en «Tus habilidades → En desarrollo» y el simulador en texto enruta al skill.

### Escenario: invocación por voz en Echo sin colisión
- Given un Echo Dot en es-ES, en la misma cuenta, con la skill habilitada
- When el usuario dice «Alexa, abre servidor personal y dime qué álbumes tengo»
- Then la skill responde con álbumes y no menciona la biblioteca de Amazon.

### Escenario: paginación tras la invocación
- Given una sesión iniciada con `servidor personal`
- When el usuario dice «dime más»
- Then `sessionAttributes.listing` contiene solo `kind` y `offset`, y los logs del webhook muestran `dispatching Alexa request` sin `401`.

### Escenario: nombres anteriores dejan de enrutar
- Given el modelo publicado con `servidor personal`
- When el usuario dice «Alexa, abre biblioteca navidrom» o «Alexa, abre reproductor navidrom»
- Then Alexa no enruta a esta skill; no es un fallo del bridge.
