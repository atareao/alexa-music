# signature Specification

## Purpose
TBD - created by archiving change fix-alexa-signature-body. Update Purpose after archive.

## Requirements

### Requirement: Verify RSA signature over raw request body
El sistema MUST verificar la firma RSA de Alexa calculando RSA-SHA256 sobre los bytes exactos del body HTTP, sin anteponer el método ni la ruta.

#### Scenario: Valid Alexa signature over raw body
- GIVEN una petición POST `/webhook` con un body JSON válido firmado por Alexa con RSA-SHA256 sobre el body
- WHEN el middleware verifica la firma
- THEN la petición MUST ser aceptada y procesada por el handler

#### Scenario: Tampered body is rejected
- GIVEN una petición con firma válida sobre un body original
- WHEN el body se modifica antes de la verificación
- THEN la verificación MUST fallar con 401

### Requirement: Prefer Signature-256 header
El sistema MUST usar la cabecera `Signature-256` (SHA-256 del body) cuando esté presente, y MUST aceptar `Signature` (SHA-1 del body) como fallback por compatibilidad.

#### Scenario: Signature-256 present
- GIVEN cabeceras `SignatureCertChainUrl` y `Signature-256` válidas
- WHEN se verifica la petición
- THEN MUST usarse RSA-SHA256 sobre el body y aceptarse

#### Scenario: Only legacy Signature present
- GIVEN cabeceras `SignatureCertChainUrl` y `Signature` con SHA-1 válido del body
- WHEN se verifica la petición
- THEN MUST usarse RSA-SHA1 sobre el body y aceptarse
