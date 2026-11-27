# List Library Intents Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Añadir intents de Alexa para listar álbumes, artistas y playlists de Navidrome, con paginación mediante “dime más”.

**Architecture:** Se añaden dos métodos de solo lectura al cliente Subsonic (`getAlbumList2`, `getArtists`) y se reutiliza `getPlaylists`. Los handlers convierten los resultados en texto y guardan un `ListingCursor` en `sessionAttributes` de Alexa, evitando estado nuevo en el servidor. La paginación se implementa con un intent `MoreListingIntent` que lee el cursor de la sesión y devuelve la siguiente página.

**Tech Stack:** Rust 1.88, Axum 0.7, Tokio 1, reqwest 0.12, serde/serde_json 1, tracing 0.1, Alexa Custom Skill es-ES, Subsonic API v1.16.1 / OpenSubsonic.

## Global Constraints

- Issue de entrada: #31 (`✨ feat: intents para listar álbumes, artistas y playlists`).
- No añadir base de datos ni almacenamiento externo; la paginación vive en `sessionAttributes` de Alexa.
- No modificar la verificación RSA ni el endpoint `/webhook`.
- No añadir `samples` a intents `AMAZON.*` en `interaction_model.json`.
- No poner `supportedInterfaces` en `interaction_model.json`; Audio Player se activa en la consola de Alexa.
- Mantener `SKIP_SIGNATURE_VERIFICATION=false` en producción.
- Todos los cambios van por rama `feature/*` desde `development`, con commits gitmoji y PR a `development` y luego a `main`.
- Verificación obligatoria: `cargo test --quiet`, `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`.

---

## File Structure

- Modify `src/subsonic.rs`: nuevos structs de respuesta y métodos `get_album_list`, `get_artists`.
- Modify `src/alexa.rs`: `Session.attributes`, `AlexaResponse.sessionAttributes`, `ListingCursor`, helpers de paginación y handlers de intents.
- Modify `interaction_model.json`: nuevos intents y samples.
- Test `src/subsonic.rs`, `src/alexa.rs`, `tests/webhook_test.rs`: tests TDD de deserialización, handlers y modelo.
- Modify `docs/alexa-signature-playlists.md`: documentar los nuevos intents.

---

## Task 1: Subsonic `getAlbumList2`

**Files:**
- Modify: `src/subsonic.rs`
- Test: `src/subsonic.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: `endpoint_url(&self, method: &str, extra: &str) -> String`, `get_json`, `ensure_ok`, `Album`.
- Produces: `pub async fn get_album_list(&self, size: usize, offset: usize) -> Result<Vec<Album>, SubsonicError>`.

- [ ] **Step 1: Write the failing test**

Añadir en `src/subsonic.rs`:

```rust
#[test]
fn get_album_list_reads_alphabetical_albums() {
    let (_base, handle) = start_mock(HashMap::from([(
        "getAlbumList2",
        r#"{"subsonic-response":{"status":"ok","version":"1.16.1","albumList2":{"album":[{"id":"a1","name":"Álbum Uno","artist":"Artista A"},{"id":"a2","name":"Álbum Dos","artist":"Artista B"}]}}}"#,
    )]));
    let client = SubsonicClient::new(_base, "user".into(), "secret".into());
    let albums = tokio_test::block_on(client.get_album_list(10, 0)).unwrap();
    assert_eq!(albums.len(), 2);
    assert_eq!(albums[0].name, "Álbum Uno");
    handle.abort();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --quiet get_album_list_reads_alphabetical_albums`

Expected: FAIL because `get_album_list` no existe.

- [ ] **Step 3: Write minimal implementation**

Añadir structs y campo en `src/subsonic.rs`:

```rust
#[derive(Debug, Deserialize)]
struct AlbumList2Body {
    #[serde(rename = "albumList2", default)]
    album_list2: AlbumList2,
}

#[derive(Debug, Deserialize)]
struct AlbumList2 {
    #[serde(default)]
    album: Vec<Album>,
}
```

Añadir a `ResponseBody`:

```rust
#[serde(default)]
pub albumList2: Option<AlbumList2Body>,
```

Añadir método:

```rust
pub async fn get_album_list(
    &self,
    size: usize,
    offset: usize,
) -> Result<Vec<Album>, SubsonicError> {
    let extra = format!("type=alphabeticalByName&size={size}&offset={offset}");
    let url = self.endpoint_url("getAlbumList2", &extra);
    let body = self.get_json(&url).await?.subsonic_response;
    ensure_ok(&body)?;
    Ok(body
        .albumList2
        .ok_or(SubsonicError::MissingField("albumList2"))?
        .album_list2
        .album)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --quiet get_album_list_reads_alphabetical_albums`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/subsonic.rs
git commit -m "✨ feat: add getAlbumList2 Subsonic client"
```

---

## Task 2: Subsonic `getArtists`

**Files:**
- Modify: `src/subsonic.rs`
- Test: `src/subsonic.rs`

**Interfaces:**
- Consumes: `endpoint_url`, `get_json`, `ensure_ok`, `Artist`.
- Produces: `pub async fn get_artists(&self) -> Result<Vec<Artist>, SubsonicError>`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn get_artists_flattens_index_entries() {
    let (_base, handle) = start_mock(HashMap::from([(
        "getArtists",
        r#"{"subsonic-response":{"status":"ok","version":"1.16.1","artists":{"index":[{"artist":[{"id":"ar1","name":"Vivaldi"}]},{"artist":[{"id":"ar2","name":"Bach"}]}]}}}"#,
    )]));
    let client = SubsonicClient::new(_base, "user".into(), "secret".into());
    let artists = tokio_test::block_on(client.get_artists()).unwrap();
    assert_eq!(artists.len(), 2);
    assert_eq!(artists[1].name, "Bach");
    handle.abort();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --quiet get_artists_flattens_index_entries`

Expected: FAIL because `get_artists` no existe.

- [ ] **Step 3: Write minimal implementation**

```rust
#[derive(Debug, Deserialize)]
struct ArtistsBody {
    #[serde(default)]
    index: Vec<ArtistIndex>,
}

#[derive(Debug, Deserialize)]
struct ArtistIndex {
    #[serde(default)]
    artist: Vec<Artist>,
}
```

Añadir a `ResponseBody`:

```rust
#[serde(default)]
pub artists: Option<ArtistsBody>,
```

Añadir método:

```rust
pub async fn get_artists(&self) -> Result<Vec<Artist>, SubsonicError> {
    let url = self.endpoint_url("getArtists", "");
    let body = self.get_json(&url).await?.subsonic_response;
    ensure_ok(&body)?;
    let artists = body.artists.ok_or(SubsonicError::MissingField("artists"))?;
    Ok(artists.index.into_iter().flat_map(|index| index.artist).collect())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --quiet get_artists_flattens_index_entries`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/subsonic.rs
git commit -m "✨ feat: add getArtists Subsonic client"
```

---

## Task 3: Session cursor and speech helper

**Files:**
- Modify: `src/alexa.rs`
- Test: `src/alexa.rs`

**Interfaces:**
- Consumes: `AlexaRequest`, `Session`, `AlexaResponse`, `ResponseBody`, `OutputSpeech`.
- Produces: `ListingCursor`, `speak_with_session`, `cursor_from_request`, `render_page`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn session_attributes_are_deserialized() {
    let payload = json!({
        "version": "1.0",
        "session": {"attributes": {"listing": {"kind":"albums","items":["A"],"offset":1}}},
        "request": {"type":"LaunchRequest","requestId":"r","timestamp":"2026-01-01T00:00:00Z","locale":"es-ES"}
    });
    let req: AlexaRequest = serde_json::from_value(payload).unwrap();
    assert!(req.session.unwrap().attributes.contains_key("listing"));
}

#[test]
fn speak_with_session_serializes_cursor() {
    let cursor = ListingCursor { kind: "albums".into(), items: vec!["A".into()], offset: 1 };
    let value = serde_json::to_value(speak_with_session("Tienes álbumes.", &cursor)).unwrap();
    assert_eq!(value["sessionAttributes"]["listing"]["kind"], "albums");
    assert_eq!(value["response"]["shouldEndSession"], false);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --quiet session_attributes_are_deserialized speak_with_session_serializes_cursor`

Expected: FAIL because fields/helpers no existen.

- [ ] **Step 3: Write minimal implementation**

```rust
use serde_json::Value;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct Session {
    #[serde(default)]
    pub sessionId: Option<String>,
    #[serde(default)]
    pub application: Option<Application>,
    #[serde(default)]
    pub user: Option<User>,
    #[serde(default)]
    pub attributes: HashMap<String, Value>,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
pub struct AlexaResponse {
    pub version: String,
    pub response: Option<ResponseBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessionAttributes: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListingCursor {
    pub kind: String,
    pub items: Vec<String>,
    pub offset: usize,
}

pub fn speak_with_session(text: &str, cursor: &ListingCursor) -> AlexaResponse {
    let mut attributes = HashMap::new();
    if let Ok(value) = serde_json::to_value(cursor) {
        attributes.insert("listing".to_owned(), value);
    }
    AlexaResponse {
        version: "1.0".to_owned(),
        response: Some(ResponseBody {
            outputSpeech: Some(OutputSpeech {
                speech_type: "PlainText".to_owned(),
                text: text.to_owned(),
            }),
            shouldEndSession: Some(false),
            ..ResponseBody::default()
        }),
        sessionAttributes: Some(attributes),
    }
}

fn cursor_from_request(req: &AlexaRequest) -> Option<ListingCursor> {
    let value = req.session.as_ref()?.attributes.get("listing")?;
    serde_json::from_value(value.clone()).ok()
}

const PAGE_SIZE: usize = 10;

fn render_page(mut cursor: ListingCursor) -> (String, ListingCursor) {
    let label = match cursor.kind.as_str() {
        "albums" => "álbumes",
        "artists" => "artistas",
        "playlists" => "listas",
        _ => "resultados",
    };
    if cursor.offset >= cursor.items.len() {
        return (format!("No hay más {label}."), cursor);
    }
    let end = (cursor.offset + PAGE_SIZE).min(cursor.items.len());
    let page = &cursor.items[cursor.offset..end];
    let text = format!("Tienes estos {label}: {}.", page.join("; "));
    cursor.offset = end;
    (text, cursor)
}
```

Actualiza todos los constructores existentes de `AlexaResponse` para incluir `sessionAttributes: None`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --quiet session_attributes_are_deserialized speak_with_session_serializes_cursor`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/alexa.rs
git commit -m "✨ feat: add Alexa session cursor for listings"
```

---

## Task 4: Listing handlers and dispatch

**Files:**
- Modify: `src/alexa.rs`
- Test: `src/alexa.rs`

**Interfaces:**
- Consumes: `SubsonicClient`, `ListingCursor`, `speak_with_session`, `cursor_from_request`, `render_page`.
- Produces: `handle_list_albums`, `handle_list_artists`, `handle_list_playlists`, `handle_more_listing`.

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn list_albums_returns_names_and_cursor() {
    let server = MockServer::start(vec![(
        "getAlbumList2",
        json!({"subsonic-response":{"status":"ok","version":"1.16.1","albumList2":{"album":[{"id":"a1","name":"Uno"},{"id":"a2","name":"Dos"}]}}}),
    )]);
    let response = handle_list_albums(&server.client()).await;
    let value = serde_json::to_value(response).unwrap();
    assert_eq!(value["sessionAttributes"]["listing"]["kind"], "albums");
    assert_eq!(value["response"]["outputSpeech"]["text"], "Tienes estos álbumes: Uno; Dos.");
}

#[tokio::test]
async fn more_listing_uses_session_cursor() {
    let payload = json!({
        "version":"1.0",
        "session":{"attributes":{"listing":{"kind":"albums","items":["Uno","Dos"],"offset":1}}},
        "request":{"type":"IntentRequest","requestId":"r","timestamp":"2026-01-01T00:00:00Z","locale":"es-ES","intent":{"name":"MoreListingIntent"}}
    });
    let req: AlexaRequest = serde_json::from_value(payload).unwrap();
    let response = handle_more_listing(&req);
    let value = serde_json::to_value(response).unwrap();
    assert_eq!(value["response"]["outputSpeech"]["text"], "Tienes estos álbumes: Dos.");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --quiet list_albums_returns_names_and_cursor more_listing_uses_session_cursor`

Expected: FAIL because handlers no existen.

- [ ] **Step 3: Write minimal implementation**

```rust
pub async fn handle_list_albums(client: &SubsonicClient) -> AlexaResponse {
    match client.get_album_list(500, 0).await {
        Ok(albums) if albums.is_empty() => speak("No tengo álbumes en la biblioteca.", true),
        Ok(albums) => {
            let items = albums.into_iter().map(|album| album.name).collect();
            let cursor = ListingCursor { kind: "albums".into(), items, offset: 0 };
            let (text, next) = render_page(cursor);
            speak_with_session(&text, &next)
        }
        Err(err) => {
            error!(error = %err, "getAlbumList2 failed");
            speak("No he podido consultar tus álbumes.", true)
        }
    }
}

pub async fn handle_list_artists(client: &SubsonicClient) -> AlexaResponse {
    match client.get_artists().await {
        Ok(artists) if artists.is_empty() => speak("No tengo artistas en la biblioteca.", true),
        Ok(artists) => {
            let items = artists.into_iter().map(|artist| artist.name).collect();
            let cursor = ListingCursor { kind: "artists".into(), items, offset: 0 };
            let (text, next) = render_page(cursor);
            speak_with_session(&text, &next)
        }
        Err(err) => {
            error!(error = %err, "getArtists failed");
            speak("No he podido consultar tus artistas.", true)
        }
    }
}

pub async fn handle_list_playlists(client: &SubsonicClient) -> AlexaResponse {
    match client.get_playlists().await {
        Ok(playlists) if playlists.is_empty() => speak("No tengo listas de reproducción.", true),
        Ok(playlists) => {
            let items = playlists.into_iter().map(|playlist| playlist.name).collect();
            let cursor = ListingCursor { kind: "playlists".into(), items, offset: 0 };
            let (text, next) = render_page(cursor);
            speak_with_session(&text, &next)
        }
        Err(err) => {
            error!(error = %err, "getPlaylists failed");
            speak("No he podido consultar tus listas.", true)
        }
    }
}

pub fn handle_more_listing(req: &AlexaRequest) -> AlexaResponse {
    let Some(cursor) = cursor_from_request(req) else {
        return speak("Dime primero qué quieres listar.", true);
    };
    let (text, next) = render_page(cursor);
    speak_with_session(&text, &next)
}
```

Añadir brazos en el `match intent.name.as_str()` de `handle_request`:

```rust
"ListAlbumsIntent" => handle_list_albums(client).await,
"ListArtistsIntent" => handle_list_artists(client).await,
"ListPlaylistsIntent" => handle_list_playlists(client).await,
"MoreListingIntent" => handle_more_listing(req),
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --quiet`

Expected: ALL PASS.

- [ ] **Step 5: Commit**

```bash
git add src/alexa.rs
git commit -m "✨ feat: add library listing intents"
```

---

## Task 5: Interaction model

**Files:**
- Modify: `interaction_model.json`
- Test: `tests/webhook_test.rs` or a small Python validation command.

**Interfaces:**
- Consumes: existing `languageModel.intents` array.
- Produces: intents `ListAlbumsIntent`, `ListArtistsIntent`, `ListPlaylistsIntent`, `MoreListingIntent`.

- [ ] **Step 1: Write the failing test**

Añadir test que lea `interaction_model.json` y verifique que existen los cuatro intents y que ningún `AMAZON.*` tiene samples.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --quiet model_declares_listing_intents`

Expected: FAIL.

- [ ] **Step 3: Write minimal implementation**

Añadir al array `intents`:

```json
{
  "name": "ListAlbumsIntent",
  "samples": [
    "qué álbumes tengo",
    "qué álbumes hay",
    "lista mis álbumes",
    "dime los álbumes",
    "dime mis álbumes"
  ]
},
{
  "name": "ListArtistsIntent",
  "samples": [
    "qué artistas tengo",
    "qué artistas hay",
    "lista mis artistas",
    "dime los artistas",
    "dime mis artistas"
  ]
},
{
  "name": "ListPlaylistsIntent",
  "samples": [
    "qué playlists tengo",
    "qué listas tengo",
    "lista mis listas",
    "dime las listas",
    "dime mis playlists"
  ]
},
{
  "name": "MoreListingIntent",
  "samples": [
    "dime más",
    "siguientes",
    "más",
    "sigue"
  ]
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --quiet model_declares_listing_intents`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add interaction_model.json tests/webhook_test.rs
git commit -m "✨ feat: declare library listing intents"
```

---

## Task 6: Docs, release and deploy

**Files:**
- Modify: `docs/alexa-signature-playlists.md`
- Modify: `HANDOFF.md`

**Interfaces:**
- Consumes: all previous tasks.
- Produces: release `v0.2.0` with image `ghcr.io/atareao/alexa-music:v0.2.0`.

- [ ] **Step 1: Update docs**

Documentar los nuevos intents, ejemplos de voz, paginación y los métodos Subsonic añadidos.

- [ ] **Step 2: Verify**

Run:

```bash
cargo test --quiet
cargo check
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```

Expected: ALL OK.

- [ ] **Step 3: Commit docs**

```bash
git add docs/alexa-signature-playlists.md HANDOFF.md
git commit -m "📝 docs: document library listing intents"
```

- [ ] **Step 4: Merge and release**

PR a `development`, PR a `main`, tag `v0.2.0`, GitHub Release.

- [ ] **Step 5: Deploy**

Actualizar `compose.yaml` en co1 a `v0.2.0`, recrear solo `alexabridge`, verificar `/health` y logs. Re-importar `interaction_model.json` en la consola de Alexa y hacer *Build Model*.

---
