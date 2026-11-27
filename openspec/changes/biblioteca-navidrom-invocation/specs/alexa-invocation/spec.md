# Delta: `alexa-invocation`

## Contratos
- `interaction_model.json.interactionModel.languageModel.invocationName` (string): debe ser exactamente `biblioteca navidrom` para el locale `es-ES`.
- Frases de invocación soportadas: `Alexa, abre biblioteca navidrom` y `Alexa, abre biblioteca navidrom y dime <petición>`.
- `sessionAttributes.listing` conserva su contrato: `{"kind": string, "offset": number}`; este cambio no lo modifica.

## Escenarios

### Escenario: el modelo declara el nuevo nombre
- Given el fichero `interaction_model.json`
- When se valida el modelo en la suite de tests
- Then el JSON es válido y `invocationName` es `biblioteca navidrom`.

### Escenario: invocación por voz entra en la skill
- Given un Echo Dot configurado en es-ES con la skill habilitada y el modelo reconstruido
- When el usuario dice «Alexa, abre biblioteca navidrom»
- Then Alexa entra en la skill y no responde «No entiendo eso».

### Escenario: petición de listado tras la invocación
- Given una sesión iniciada con el nuevo nombre de invocación
- When el usuario dice «qué álbumes tengo» y después «dime más»
- Then la respuesta contiene álbumes y `sessionAttributes.listing` contiene solo `kind` y `offset`.

### Escenario: el nombre antiguo deja de enrutar a esta skill
- Given el modelo publicado con `biblioteca navidrom`
- When el usuario dice «Alexa, abre biblioteca musical»
- Then Alexa no enruta a esta skill y puede resolver a Amazon Music; no es un fallo del bridge.
