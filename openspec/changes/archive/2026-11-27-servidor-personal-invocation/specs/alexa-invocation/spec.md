# Delta: `alexa-invocation-servidor`

## ADDED Requirements

### Requirement: El modelo declara el nombre de invocación `servidor personal`
El sistema MUST declarar `servidor personal` como `invocationName` del modelo es-ES para que Alexa enrute las peticiones a este skill.

- `interaction_model.json.interactionModel.languageModel.invocationName` (string): debe ser exactamente `servidor personal` para `es-ES`.
- Frases soportadas: `Alexa, abre servidor personal` y `Alexa, abre servidor personal y dime <petición>`.
- `sessionAttributes.listing` conserva su contrato: `{"kind": string, "offset": number}`; este cambio no lo modifica.

#### Scenario: el modelo declara el nuevo nombre
- Given el fichero `interaction_model.json`
- When se valida el modelo en la suite de tests
- Then el JSON es válido y `invocationName` es `servidor personal`.

### Requirement: La skill disponible en Development
El skill MUST aparecer habilitado en modo *Development* para que el dispositivo y el simulador lo enruten.

#### Scenario: prueba habilitada en Development
- Given la consola Alexa con el modelo reconstruido
- When se activa *Skill testing is enabled in = Development* y se habilita la skill en la app Alexa
- Then la skill aparece en «Tus habilidades → En desarrollo» y el simulador en texto enruta al skill.

### Requirement: Invocación por voz y paginación en Echo
El skill MUST responder a la invocación por voz sin colisiones con funcionalidades integradas de Alexa y MUST conservar el cursor de paginación.

#### Scenario: invocación por voz en Echo sin colisión
- Given un Echo Dot en es-ES, en la misma cuenta, con la skill habilitada
- When el usuario dice «Alexa, abre servidor personal y dime qué álbumes tengo»
- Then la skill responde con álbumes y no menciona la biblioteca de Amazon.

#### Scenario: paginación tras la invocación
- Given una sesión iniciada con `servidor personal`
- When el usuario dice «dime más»
- Then `sessionAttributes.listing` contiene solo `kind` y `offset`, y los logs del webhook muestran `dispatching Alexa request` sin `401`.

#### Scenario: nombres anteriores dejan de enrutar
- Given el modelo publicado con `servidor personal`
- When el usuario dice «Alexa, abre biblioteca navidrom» o «Alexa, abre reproductor navidrom»
- Then Alexa no enruta a esta skill; no es un fallo del bridge.
