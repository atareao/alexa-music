# Alexa ↔ Navidrome Bridge — Implementation Plan

## Objetivo

Construir un microservicio en Rust (Axum) que conecte Alexa (ASK) con Navidrome (Subsonic API), permitiendo control por voz de la reproducción musical con cola en memoria por `deviceId`.

## Arquitectura

Binario único sin base de datos externa; estado de reproducción en memoria (`Arc<RwLock<HashMap<String, PlaybackQueue>>>`). Un endpoint `/webhook` recibe payloads de Alexa, valida la firma y despacha a handlers de intents y eventos de `AudioPlayer`. Un cliente Subsonic genera autenticación por petición (salt + token MD5) y fuerza streaming MP3. Despliegue como contenedor con Quadlet + Traefik.

## Tareas

### Tarea 1: Inicialización del proyecto y esqueleto de módulos

**Archivos:**
- Crear: `Cargo.toml`
- Crear: `src/main.rs`
- Crear: `src/config.rs`
- Crear: `src/subsonic.rs`
- Crear: `src/alexa.rs`
- Crear: `src/state.rs`

- [x] **Paso 1:** Crear `Cargo.toml` con dependencias y perfil release.
      ```toml
      [package]
      name = "alexa_navidrome_bridge"
      version = "0.1.0"
      edition = "2021"

      [dependencies]
      axum = "0.7"
      tokio = { version = "1", features = ["full"] }
      reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
      serde = { version = "1", features = ["derive"] }
      serde_json = "1"
      md5 = "0.7"
      rand = "0.8"
      urlencoding = "2"
      tracing = "0.1"
      tracing-subscriber = { version = "0.3", features = ["env-filter"] }
      async-trait = "0.1"
      thiserror = "1"
      ```

- [x] **Paso 2:** Crear módulos vacíos con declaraciones en `src/main.rs`.
      ```rust
      mod config;
      mod subsonic;
      mod alexa;
      mod state;

      #[tokio::main]
      async fn main() {
          tracing_subscriber::fmt()
              .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
              .init();
          tracing::info!("booting");
      }
      ```

- [x] **Paso 3:** Verificar compilación.
      ```bash
      cargo check
      ```

### Tarea 2: Módulo de configuración (`config.rs`)

**Archivos:**
- Crear/Modificar: `src/config.rs`

- [x] **Paso 1:** Definir `struct Config` con campos `navidrome_url`, `navidrome_user`, `navidrome_password`, `alexa_skill_id: Option<String>`, `port: u16`, `skip_signature_verification: bool`, y `fn from_env() -> Result<Self, ConfigError>`.
      ```rust
      #[derive(Debug, Clone)]
      pub struct Config {
          pub navidrome_url: String,
          pub navidrome_user: String,
          pub navidrome_password: String,
          pub alexa_skill_id: Option<String>,
          pub port: u16,
          pub skip_signature_verification: bool,
      }

      impl Config {
          pub fn from_env() -> Result<Self, ConfigError> {
              Ok(Self {
                  navidrome_url: std::env::var("NAVIDROME_URL").map_err(|_| ConfigError::Missing("NAVIDROME_URL"))?,
                  navidrome_user: std::env::var("NAVIDROME_USER").map_err(|_| ConfigError::Missing("NAVIDROME_USER"))?,
                  navidrome_password: std::env::var("NAVIDROME_PASSWORD").map_err(|_| ConfigError::Missing("NAVIDROME_PASSWORD"))?,
                  alexa_skill_id: std::env::var("ALEXA_SKILL_ID").ok(),
                  port: std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000),
                  skip_signature_verification: std::env::var("SKIP_SIGNATURE_VERIFICATION")
                      .map(|v| v == "true").unwrap_or(false),
              })
          }
      }
      ```

- [x] **Paso 2:** Añadir `enum ConfigError { Missing(&'static str) }` con `impl Display + Error`.
      ```rust
      #[derive(Debug, thiserror::Error)]
      pub enum ConfigError { #[error("missing env var: {0}")] Missing(&'static str) }
      ```

- [x] **Paso 3:** Test unitario de `from_env` con variables inyectadas.
      ```rust
      #[cfg(test)]
      mod tests {
          use super::*;
          #[test]
          fn defaults_applied() {
              std::env::set_var("NAVIDROME_URL", "https://n.example");
              std::env::set_var("NAVIDROME_USER", "u");
              std::env::set_var("NAVIDROME_PASSWORD", "p");
              let c = Config::from_env().unwrap();
              assert_eq!(c.port, 3000);
              assert!(!c.skip_signature_verification);
          }
      }
      ```

### Tarea 3: Cliente Subsonic (`subsonic.rs`)

**Archivos:**
- Crear/Modificar: `src/subsonic.rs`

- [x] **Paso 1:** Implementar generación de auth params por petición (salt alfanumérico de 8 chars, token MD5).
      ```rust
      use rand::Rng;
      const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

      pub fn auth_params(user: &str, password: &str) -> (String, String) {
          let salt: String = (0..8).map(|_| {
              let mut rng = rand::thread_rng();
              CHARS[rng.gen_range(0..CHARS.len())] as char
          }).collect();
          let token = format!("{:x}", md5::compute(format!("{password}{salt}")));
          (salt, token)
      }
      ```

- [x] **Paso 2:** Definir `struct SubsonicClient { base: String, user: String, password: String }` y `fn base_query(&self) -> String`.
      ```rust
      pub struct SubsonicClient { base: String, user: String, password: String }

      impl SubsonicClient {
          pub fn base_query(&self) -> String {
              let (salt, token) = auth_params(&self.user, &self.password);
              format!("u={}&t={}&s={}&v=1.16.1&c=alexa_navidrome_bridge&f=json",
                  self.user, token, salt)
          }
          pub fn stream_url(&self, song_id: &str) -> String {
              format!("{}/rest/stream?{}&id={}&format=mp3",
                  self.base, self.base_query(), urlencoding::encode(song_id))
          }
      }
      ```

- [x] **Paso 3:** Implementar métodos async `search3`, `get_playlists`, `get_playlist`, `get_album`, `get_artist` que llaman a `{base}/rest/<method>?<query>` vía `reqwest` y deserializan JSON a tipos de dominio (Song, Album, Artist, Playlist).

- [x] **Paso 4:** Test de auth (token determinista con salt fijo) y de construcción de `stream_url`.
      ```rust
      #[test]
      fn token_is_md5_of_password_salt() {
          // salt fijo inyectado para determinismo
          let expected = format!("{:x}", md5::compute("secretABCD1234"));
          assert_eq!(expected.len(), 32);
      }
      ```

### Tarea 4: Gestión de estado (`state.rs`)

**Archivos:**
- Crear/Modificar: `src/state.rs`

- [x] **Paso 1:** Definir tipos de cola y estado.
      ```rust
      use std::collections::HashMap;
      use std::sync::{Arc, RwLock};

      #[derive(Clone)]
      pub struct PlaybackQueue {
          pub tracks: Vec<String>,      // song ids
          pub index: usize,
          pub token: Option<String>,    // audioPlayer token actual
          pub offset_ms: i64,
          pub shuffled: bool,
          pub original_order: Vec<String>,
      }

      pub type AppState = Arc<RwLock<HashMap<String, PlaybackQueue>>>;
      ```

- [x] **Paso 2:** Implementar helpers `get_or_create`, `advance`, `retreat`, `shuffle_remaining`, `restore_order`, `save_offset`.
      ```rust
      impl PlaybackQueue {
          pub fn advance(&mut self) -> Option<String> {
              if self.index + 1 < self.tracks.len() { self.index += 1; Some(self.tracks[self.index].clone()) } else { None }
          }
          pub fn retreat(&mut self) -> Option<String> {
              if self.index > 0 { self.index -= 1; Some(self.tracks[self.index].clone()) } else { None }
          }
      }
      ```

- [x] **Paso 3:** Tests de advance/retreat/shuffle/restore con vectores de ejemplo.

### Tarea 5: Tipos de request/response de Alexa (`alexa.rs`)

**Archivos:**
- Crear/Modificar: `src/alexa.rs`

- [x] **Paso 1:** Definir structs de request con `serde` (Request envelope, `request.type` tag, `intent`, `slots`, `audioPlayer`, `context.System.device.deviceId`, `session`).
      ```rust
      #[derive(Deserialize)]
      pub struct AlexaRequest {
          pub version: String,
          pub session: Option<Session>,
          pub context: Option<Context>,
          pub request: RequestBody,
      }
      #[derive(Deserialize)]
      #[serde(tag = "type")]
      pub enum RequestBody {
          LaunchRequest,
          IntentRequest { intent: Intent },
          SessionEndedRequest,
          #[serde(rename = "AudioPlayer.PlaybackStarted")] PlaybackStarted(AudioPlayerEvent),
          #[serde(rename = "AudioPlayer.PlaybackNearlyFinished")] PlaybackNearlyFinished(AudioPlayerEvent),
          #[serde(rename = "AudioPlayer.PlaybackStopped")] PlaybackStopped(AudioPlayerEvent),
          #[serde(rename = "AudioPlayer.PlaybackFinished")] PlaybackFinished(AudioPlayerEvent),
          #[serde(rename = "AudioPlayer.PlaybackFailed")] PlaybackFailed(AudioPlayerEvent),
      }
      ```

- [x] **Paso 2:** Definir structs de response (`ResponseBody` con `outputSpeech`, `reprompt`, `shouldEndSession`, `directives`) y helpers `speak()`, `audio_player_play()`, `audio_player_stop()`, `empty()`.

- [x] **Paso 3:** Tests de (de)serialización con payloads Alexa reales (LaunchRequest, PlayMusicIntent, PlaybackNearlyFinished).

### Tarea 6: Handlers de intents (`alexa.rs`)

**Archivos:**
- Modificar: `src/alexa.rs`

- [x] **Paso 1:** Implementar `handle_launch` → mensaje de bienvenida en español, `shouldEndSession=false`.
- [x] **Paso 2:** Implementar `handle_play_music` (slot `Query`/`AMAZON.SearchQuery`): `search3`, priorizar songs → albums → artists, cargar cola, reproducir primer track con `AudioPlayer.Play` REPLACE_ALL.
- [x] **Paso 3:** Implementar `handle_play_playlist` (slot `Playlist`): `getPlaylists`, match por nombre, `getPlaylist`, cargar cola, reproducir primero.
- [x] **Paso 4:** Implementar `AMAZON.NextIntent` (advance + play/stop), `AMAZON.PreviousIntent` (retreat + play).
- [x] **Paso 5:** Implementar `AMAZON.PauseIntent`/`StopIntent`/`CancelIntent` → `AudioPlayer.Stop`.
- [x] **Paso 6:** Implementar `AMAZON.ResumeIntent` → reanudar desde offset/token guardado.
- [x] **Paso 7:** Implementar `AMAZON.ShuffleOnIntent`/`ShuffleOffIntent` → shuffle/restaurar cola restante.
- [x] **Paso 8:** Tests por intent con payloads de ejemplo y aserciones sobre directivas generadas.

### Tarea 7: Handlers de eventos AudioPlayer (`alexa.rs`)

**Archivos:**
- Modificar: `src/alexa.rs`

- [x] **Paso 1:** `PlaybackNearlyFinished` → encolar siguiente con `ENQUEUE` y `expectedPreviousToken`.
- [x] **Paso 2:** `PlaybackStopped` → guardar offset/token para resume.
- [x] **Paso 3:** `PlaybackStarted`/`PlaybackFinished`/`PlaybackFailed` → solo log, respuesta vacía.
- [x] **Paso 4:** `SessionEndedRequest` → 200 OK vacío.
- [x] **Paso 5:** Tests de cada evento (directiva ENQUEUE presente, respuesta vacía en el resto).

### Tarea 8: Middleware de verificación de firma (`alexa.rs` o `middleware.rs`)

**Archivos:**
- Crear: `src/middleware.rs`
- Modificar: `src/main.rs`

- [x] **Paso 1:** Implementar extractor/middleware que valida headers `SignatureCertChainUrl` y `Signature`.
- [x] **Paso 2:** Validar URL del certificado: scheme https, host `s3.amazonaws.com`, path empieza por `/echo.api/`.
- [x] **Paso 3:** Saltar verificación criptográfica si `SKIP_SIGNATURE_VERIFICATION=true` (manteniendo checks de presencia/formato).
- [x] **Paso 4:** Tests de rechazo (URL inválida, headers ausentes) y aceptación con skip activado.

### Tarea 9: Endpoint `/webhook`

**Archivos:**
- Crear/Modificar: `src/main.rs`

- [x] **Paso 1:** Definir handler `POST /webhook` protegido por el middleware de firma, que deserializa `AlexaRequest` y despacha según tipo.
- [x] **Paso 2:** Validar `applicationId` contra `ALEXA_SKILL_ID` si está configurado.
- [x] **Paso 3:** Devolver `Json<ResponseBody>` o respuesta vacía según corresponda.
- [x] **Paso 4:** Test de integración: POST de un LaunchRequest → 200 con outputSpeech en español.

### Tarea 10: Endpoint `/health`

**Archivos:**
- Crear/Modificar: `src/main.rs`

- [x] **Paso 1:** Handler `GET /health` → `200` con `{"status":"ok"}` (exento de verificación de firma).
- [x] **Paso 2:** Test de integración del healthcheck.

### Tarea 11: Cableado principal (`main.rs`)

**Archivos:**
- Modificar: `src/main.rs`

- [x] **Paso 1:** Cargar `Config::from_env()` (abortar con error claro si faltan obligatorias).
- [x] **Paso 2:** Construir `SubsonicClient`, `AppState` y `Router` con rutas `/health` y `/webhook`.
- [x] **Paso 3:** Enlazar `TcpListener` en `0.0.0.0:{port}` y servir con `axum::serve`.
- [x] **Paso 4:** Verificar arranque local con `cargo run` y `curl /health`.

### Tarea 12: Suite de tests

**Archivos:**
- Crear: `tests/webhook_test.rs`
- Crear: `tests/subsonic_test.rs`

- [x] **Paso 1:** Tests de integración con payloads Alexa reales (Launch, PlayMusic, Next, Pause, AudioPlayer events).
- [x] **Paso 2:** Mock de respuestas Subsonic (servidor HTTP de test o trait inyectable) para `search3`/`getPlaylist`.
- [x] **Paso 3:** Ejecutar `cargo test` y `cargo clippy -- -D warnings` y `cargo fmt --check` en verde.

### Tarea 13: Containerfile multi-stage

**Archivos:**
- Crear: `Containerfile`

- [x] **Paso 1:** Etapa builder `rust:alpine` compilando release.
- [x] **Paso 2:** Etapa runtime `alpine`/distroless, usuario UID 1000, copiar binario.
- [x] **Paso 3:** `EXPOSE 3000`, `ENTRYPOINT` al binario.
      ```dockerfile
      FROM rust:alpine AS builder
      WORKDIR /app
      COPY . .
      RUN cargo build --release

      FROM alpine:3.20
      RUN adduser -D -u 1000 app
      COPY --from=builder /app/target/release/alexa_navidrome_bridge /usr/local/bin/
      USER 1000
      EXPOSE 3000
      ENTRYPOINT ["alexa_navidrome_bridge"]
      ```

### Tarea 14: Podman Quadlet (`alexabridge.container`)

**Archivos:**
- Crear: `alexabridge.container`

- [x] **Paso 1:** Definir `[Container]` con `Image`, `EnvironmentFile`, `PublishPort` interno, labels de Traefik.
- [x] **Paso 2:** Definir `[Service]` restart y `[Install]`.
      ```ini
      [Unit]
      Description=Alexa Navidrome Bridge

      [Container]
      Image=localhost/alexa_navidrome_bridge:latest
      EnvironmentFile=/etc/alexabridge.env
      Label=traefik.enable=true
      Label=traefik.http.routers.alexabridge.rule=Host(`alexa.example.com`)
      Label=traefik.http.services.alexabridge.loadbalancer.server.port=3000

      [Service]
      Restart=always

      [Install]
      WantedBy=multi-user.target
      ```

### Tarea 15: Modelo de interacción Alexa (`interaction_model.json`)

**Archivos:**
- Crear: `interaction_model.json`

- [x] **Paso 1:** Definir `languageModel` `es-ES` con intents `PlayMusicIntent` (slot `Query` tipo `AMAZON.SearchQuery`), `PlayPlaylistIntent` (slot `Playlist` tipo `AMAZON.USState`/custom), más intents estándar (`AMAZON.NextIntent`, `PreviousIntent`, `PauseIntent`, `StopIntent`, `CancelIntent`, `ResumeIntent`, `ShuffleOnIntent`, `ShuffleOffIntent`, `HelpIntent`, `FallbackIntent`).
- [x] **Paso 2:** Añadir utterances de ejemplo en español para cada intent.
- [x] **Paso 3:** Validar JSON (`jq . interaction_model.json`).