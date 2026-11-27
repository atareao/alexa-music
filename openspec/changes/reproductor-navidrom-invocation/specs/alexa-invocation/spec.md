# Delta: `alexa-invocation-reproductor`

## Contratos
- `interaction_model.json.interactionModel.languageModel.invocationName` (string): debe ser exactamente `reproductor navidrom` para `es-ES`.
- Frases soportadas: `Alexa, abre reproductor navidrom` y `Alexa, abre reproductor navidrom y dime <petición>`.
- `sessionAttributes.listing` conserva su contrato: `{"kind": string, "offset": number}`; este cambio no lo modifica.

## Escenarios

### Escenario: el modelo declara el nuevo nombre
- Given el fichero `interaction_model.json`
- When se valida el modelo en la suite de tests
- Then el JSON es válido y `invocationName` es `reproductor navidrom`.

### Escenario: el simulador en texto enruta al skill
- Given la consola Alexa con el modelo reconstruido
- When se escribe `abre reproductor navidrom`
- Then responde este skill y no la biblioteca musical integrada.

### Escenario: invocación por voz en Echo sin colisión
- Given un Echo Dot en es-ES con la skill activa y el modelo reconstruido
- When el usuario dice «Alexa, abre reproductor navidrom y dime qué álbumes tengo»
- Then la skill responde con álbumes y no dice «No encontré ninguna biblioteca Navidrome».

### Escenario: paginación tras la invocación
- Given una sesión iniciada con `reproductor navidrom`
- When el usuario dice «dime más»
- Then `sessionAttributes.listing` contiene solo `kind` y `offset`.

### Escenario: el nombre anterior deja de enrutar
- Given el modelo publicado con `reproductor navidrom`
- When el usuario dice «Alexa, abre biblioteca navidrom»
- Then Alexa no enruta a esta skill; no es un fallo del bridge.
