//! Read-only Subsonic (Navidrome) client used by the Alexa bridge.
//!
//! Authentication follows the Subsonic password-encoding scheme: every
//! request carries a fresh random salt and `md5(password + salt)` as token.

use std::collections::HashSet;
use std::time::Duration;

use rand::Rng;
use serde::Deserialize;
use tracing::{debug, warn};

const SALT_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const API_VERSION: &str = "1.16.1";
const CLIENT_NAME: &str = "alexa_navidrome_bridge";

/// Errors produced while talking to a Subsonic server.
#[derive(Debug, thiserror::Error)]
pub enum SubsonicError {
    /// Transport error or non-success HTTP status.
    #[error("Subsonic HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    /// The response body was not valid JSON for the expected shape.
    #[error("Subsonic response deserialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    /// The server reported `status=failed` (or another unexpected status).
    #[error("Subsonic API error: {message}")]
    Api { message: String },
    /// An expected list was absent from an otherwise successful response.
    #[error("missing field in Subsonic response: {0}")]
    MissingField(&'static str),
}

/// A single track.
#[derive(Debug, Clone, Deserialize)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub artist: String,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub duration: Option<i64>,
}

/// An album, optionally expanded with its tracks.
#[derive(Debug, Clone, Deserialize)]
pub struct Album {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub song: Option<Vec<Song>>,
}

/// An artist summary returned by `search3`.
#[derive(Debug, Clone, Deserialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
}

/// A playlist summary returned by `getPlaylists`.
// The Subsonic JSON key is `songCount`; keeping the field name identical
// avoids a serde rename while mirroring the API contract exactly.
#[allow(non_snake_case)]
#[derive(Debug, Clone, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub songCount: Option<i64>,
}

/// Normalized result of a `search3` call.
#[derive(Debug, Clone, Default)]
pub struct Search3Result {
    pub songs: Vec<Song>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
}

/// Envelope that wraps every Subsonic response.
#[derive(Debug, Clone, Deserialize)]
pub struct SubsonicResponse {
    #[serde(rename = "subsonic-response")]
    pub subsonic_response: ResponseBody,
}

/// Body of the `subsonic-response` envelope.
///
/// The `get*` fields are named after the Subsonic methods and are mapped to
/// the actual JSON keys (`playlists`, `playlist`, `album`, `artist`,
/// `topSongs`). Every field is optional so partial mocks deserialize safely.
#[allow(non_snake_case)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResponseBody {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub error: Option<SubsonicApiError>,
    #[serde(default)]
    pub searchResult3: Option<SearchResult3Body>,
    #[serde(default, rename = "playlists")]
    pub getPlaylists: Option<PlaylistsBody>,
    #[serde(default, rename = "playlist")]
    pub getPlaylist: Option<PlaylistBody>,
    #[serde(default, rename = "album")]
    pub getAlbum: Option<AlbumBody>,
    #[serde(default, rename = "artist")]
    pub getArtist: Option<GetArtistBody>,
    #[serde(default, rename = "topSongs")]
    pub getTopSongs: Option<TopSongsBody>,
}

/// Error detail included in a failed Subsonic response.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SubsonicApiError {
    #[serde(default)]
    pub code: Option<i64>,
    #[serde(default)]
    pub message: Option<String>,
}

/// `searchResult3` payload.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SearchResult3Body {
    #[serde(default)]
    pub song: Option<Vec<Song>>,
    #[serde(default)]
    pub album: Option<Vec<Album>>,
    #[serde(default)]
    pub artist: Option<Vec<Artist>>,
}

/// `playlists` payload.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PlaylistsBody {
    #[serde(default)]
    pub playlist: Option<Vec<Playlist>>,
}

/// Tracks of a playlist returned by `getPlaylist`.
///
/// Navidrome 0.64+ (OpenSubsonic) uses `entry`, while other Subsonic servers
/// use `song`; both keys are accepted.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PlaylistBody {
    #[serde(default)]
    pub song: Option<Vec<Song>>,
    #[serde(default)]
    pub entry: Option<Vec<Song>>,
}

impl PlaylistBody {
    /// Returns all tracks, with `entry` first and duplicate ids removed.
    pub fn songs(self) -> Vec<Song> {
        let mut seen: HashSet<String> = HashSet::new();
        self.entry
            .unwrap_or_default()
            .into_iter()
            .chain(self.song.unwrap_or_default())
            .filter(|song| seen.insert(song.id.clone()))
            .collect()
    }
}

/// Tracks of an album returned by `getAlbum`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AlbumBody {
    #[serde(default)]
    pub song: Option<Vec<Song>>,
}

/// Artist detail returned by `getArtist` (only albums are relevant here).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GetArtistBody {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub album: Option<Vec<Album>>,
}

/// Tracks returned by `getTopSongs`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TopSongsBody {
    #[serde(default)]
    pub song: Option<Vec<Song>>,
}

/// Generates a fresh 8-character alphanumeric salt and its auth token.
pub fn auth_params(user: &str, password: &str) -> (String, String) {
    let salt: String = {
        let mut rng = rand::thread_rng();
        (0..8)
            .map(|_| SALT_CHARS[rng.gen_range(0..SALT_CHARS.len())] as char)
            .collect()
    };
    auth_params_with_salt(user, password, &salt)
}

/// Computes `(salt, token)` from a caller-supplied salt.
///
/// `user` is part of the signature to keep the helpers symmetric; Subsonic's
/// token is `hex(md5(password + salt))` and deliberately excludes the user.
pub fn auth_params_with_salt(_user: &str, password: &str, salt: &str) -> (String, String) {
    let token = format!("{:x}", md5::compute(format!("{password}{salt}")));
    (salt.to_owned(), token)
}

/// Client for the read-only Subsonic endpoints needed by the bridge.
#[derive(Debug, Clone)]
pub struct SubsonicClient {
    base: String,
    user: String,
    password: String,
    http: reqwest::Client,
}

impl SubsonicClient {
    /// Creates a client for the given Navidrome base URL and credentials.
    pub fn new(base: String, user: String, password: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|err| {
                warn!(
                    error = %err,
                    "could not build Subsonic client with timeout; using default client"
                );
                reqwest::Client::new()
            });
        Self {
            base,
            user,
            password,
            http,
        }
    }

    /// Builds the shared authentication query string with a fresh salt.
    pub fn base_query(&self) -> String {
        let (salt, token) = auth_params(&self.user, &self.password);
        format!(
            "u={}&t={}&s={}&v={}&c={}&f=json",
            urlencoding::encode(&self.user),
            token,
            urlencoding::encode(&salt),
            API_VERSION,
            CLIENT_NAME
        )
    }

    /// Builds a signed URL that streams `song_id` as MP3 for Alexa.
    pub fn stream_url(&self, song_id: &str) -> String {
        format!(
            "{}/rest/stream?{}&id={}&format=mp3",
            self.base.trim_end_matches('/'),
            self.base_query(),
            urlencoding::encode(song_id)
        )
    }

    /// Builds a URL for a REST method with optional extra query parameters.
    fn endpoint_url(&self, method: &str, extra: &str) -> String {
        let base = format!(
            "{}/rest/{}?{}",
            self.base.trim_end_matches('/'),
            method,
            self.base_query()
        );
        if extra.is_empty() {
            base
        } else {
            format!("{base}&{extra}")
        }
    }

    /// Gets and parses a Subsonic envelope, checking the HTTP status first.
    async fn get_json(&self, url: &str) -> Result<SubsonicResponse, SubsonicError> {
        let response = self.http.get(url).send().await?.error_for_status()?;
        let bytes = response.bytes().await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    /// Searches songs, albums and artists with `search3`.
    pub async fn search3(&self, query: &str) -> Result<Search3Result, SubsonicError> {
        let url = self.endpoint_url("search3", &format!("query={}", urlencoding::encode(query)));
        let body = self.get_json(&url).await?.subsonic_response;
        ensure_ok(&body)?;
        let result = body
            .searchResult3
            .ok_or(SubsonicError::MissingField("searchResult3"))?;
        Ok(Search3Result {
            songs: result.song.unwrap_or_default(),
            albums: result.album.unwrap_or_default(),
            artists: result.artist.unwrap_or_default(),
        })
    }

    /// Lists all playlists.
    pub async fn get_playlists(&self) -> Result<Vec<Playlist>, SubsonicError> {
        let url = self.endpoint_url("getPlaylists", "");
        let body = self.get_json(&url).await?.subsonic_response;
        ensure_ok(&body)?;
        Ok(body
            .getPlaylists
            .ok_or(SubsonicError::MissingField("playlists"))?
            .playlist
            .unwrap_or_default())
    }

    /// Lists the songs of a playlist.
    pub async fn get_playlist(&self, id: &str) -> Result<Vec<Song>, SubsonicError> {
        let url = self.endpoint_url("getPlaylist", &format!("id={}", urlencoding::encode(id)));
        let body = self.get_json(&url).await?.subsonic_response;
        ensure_ok(&body)?;
        Ok(body
            .getPlaylist
            .ok_or(SubsonicError::MissingField("playlist"))?
            .songs())
    }

    /// Lists the songs of an album.
    pub async fn get_album(&self, id: &str) -> Result<Vec<Song>, SubsonicError> {
        let url = self.endpoint_url("getAlbum", &format!("id={}", urlencoding::encode(id)));
        let body = self.get_json(&url).await?.subsonic_response;
        ensure_ok(&body)?;
        Ok(body
            .getAlbum
            .ok_or(SubsonicError::MissingField("album"))?
            .song
            .unwrap_or_default())
    }

    /// Lists the most popular songs of an artist via `getTopSongs`.
    pub async fn get_artist_top_songs(&self, id: &str) -> Result<Vec<Song>, SubsonicError> {
        let url = self.endpoint_url(
            "getTopSongs",
            &format!("artist={}", urlencoding::encode(id)),
        );
        let body = self.get_json(&url).await?.subsonic_response;
        ensure_ok(&body)?;
        if let Some(artist) = &body.getArtist {
            debug!(
                artist_id = ?artist.id,
                artist = ?artist.name,
                albums = artist.album.as_ref().map_or(0, Vec::len),
                "artist details present in response"
            );
        }
        Ok(body
            .getTopSongs
            .ok_or(SubsonicError::MissingField("topSongs"))?
            .song
            .unwrap_or_default())
    }
}

/// Rejects a Subsonic response whose status is not exactly `ok`.
fn ensure_ok(body: &ResponseBody) -> Result<(), SubsonicError> {
    match body.status.as_deref() {
        Some("ok") => Ok(()),
        Some(status) => {
            let code = body.error.as_ref().and_then(|error| error.code);
            let message = body
                .error
                .as_ref()
                .and_then(|error| error.message.clone())
                .unwrap_or_else(|| match code {
                    Some(code) => format!("Subsonic error {code} (status: {status})"),
                    None => format!("unexpected Subsonic status: {status}"),
                });
            Err(SubsonicError::Api { message })
        }
        None => Err(SubsonicError::Api {
            message: "missing status in Subsonic response".to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Minimal one-shot HTTP server serving a fixed `getPlaylist` response.
    struct MockPlaylistServer {
        base: String,
        task: tokio::task::JoinHandle<()>,
    }

    impl MockPlaylistServer {
        async fn start(response: &'static str) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind mock Subsonic server");
            let addr = listener.local_addr().expect("mock server address");
            let task = tokio::spawn(async move {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let mut request = Vec::new();
                let mut chunk = [0_u8; 1024];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    let read = socket.read(&mut chunk).await.unwrap_or(0);
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..read]);
                }
                let payload = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.len(),
                    response
                );
                let _ = socket.write_all(payload.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
            Self {
                base: format!("http://{addr}"),
                task,
            }
        }

        fn client(&self) -> SubsonicClient {
            SubsonicClient::new(self.base.clone(), "user".into(), "password".into())
        }
    }

    impl Drop for MockPlaylistServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    #[tokio::test]
    async fn get_playlist_reads_navidrome_entry_tracks() {
        let server = MockPlaylistServer::start(
            r#"{"subsonic-response":{"status":"ok","playlist":{"id":"p1","name":"clasica","songCount":1,"entry":[{"id":"s1","title":"T","artist":"A"}]}}}"#,
        )
        .await;

        let songs = server
            .client()
            .get_playlist("p1")
            .await
            .expect("playlist tracks");

        assert_eq!(songs.len(), 1, "songs: {songs:?}");
        assert_eq!(songs[0].id, "s1");
    }

    #[tokio::test]
    async fn get_playlist_still_reads_legacy_song_tracks() {
        let server = MockPlaylistServer::start(
            r#"{"subsonic-response":{"status":"ok","playlist":{"id":"p1","name":"Fav","song":[{"id":"s2","title":"T2","artist":"A"}]}}}"#,
        )
        .await;

        let songs = server
            .client()
            .get_playlist("p1")
            .await
            .expect("playlist tracks");

        assert_eq!(songs.len(), 1, "songs: {songs:?}");
        assert_eq!(songs[0].id, "s2");
    }

    #[tokio::test]
    async fn get_playlist_merges_entry_and_song_without_duplicates() {
        let server = MockPlaylistServer::start(
            r#"{"subsonic-response":{"status":"ok","playlist":{"id":"p1","name":"Both","entry":[{"id":"s1","title":"T1","artist":"A"}],"song":[{"id":"s1","title":"T1","artist":"A"},{"id":"s2","title":"T2","artist":"A"}]}}}"#,
        )
        .await;

        let songs = server
            .client()
            .get_playlist("p1")
            .await
            .expect("playlist tracks");

        assert_eq!(songs.len(), 2, "songs: {songs:?}");
        assert_eq!(songs[0].id, "s1");
        assert_eq!(songs[1].id, "s2");
    }

    #[test]
    fn token_is_md5_of_password_salt() {
        let (salt, token) = auth_params_with_salt("user", "secret", "ABCD1234");

        assert_eq!(salt, "ABCD1234");
        assert_eq!(token.len(), 32);
        assert!(token
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
        assert_eq!(token, "b9484826ef04b164e1bbf2e1c6206fdb");
    }

    #[test]
    fn stream_url_forces_mp3() {
        let client = SubsonicClient::new("https://n.example".into(), "user".into(), "pw".into());
        let url = client.stream_url("tr/ack 1");

        assert!(url.starts_with("https://n.example/rest/stream?"));
        assert!(url.contains("format=mp3"));
        assert!(url.contains("id=tr%2Fack%201"));
    }

    #[test]
    fn base_query_uses_fresh_salt_and_required_params() {
        let client = SubsonicClient::new("https://n.example".into(), "a b".into(), "pw".into());
        let first = client.base_query();
        let second = client.base_query();

        assert!(first.contains("u=a%20b"));
        assert!(first.contains("v=1.16.1"));
        assert!(first.contains("c=alexa_navidrome_bridge"));
        assert!(first.contains("f=json"));
        assert_ne!(first, second);
    }

    #[test]
    fn search3_response_deserializes() {
        let json = r#"{
            "subsonic-response": {
                "status": "ok",
                "searchResult3": {
                    "song": [{"id":"s1","title":"Song","artist":"Artist","duration":42}],
                    "album": [{"id":"a1","name":"Album"}],
                    "artist": [{"id":"r1","name":"Artist"}]
                }
            }
        }"#;

        let envelope: SubsonicResponse = serde_json::from_str(json).unwrap();
        let body = envelope.subsonic_response;
        let result = body.searchResult3.unwrap();

        assert_eq!(result.song.unwrap()[0].duration, Some(42));
        assert_eq!(result.album.unwrap()[0].name, "Album");
        assert_eq!(result.artist.unwrap()[0].id, "r1");
    }

    #[test]
    fn list_responses_deserialize() {
        let playlists: SubsonicResponse = serde_json::from_str(
            r#"{"subsonic-response":{"status":"ok","playlists":{"playlist":[{"id":"p1","name":"Fav","songCount":7}]}}}"#,
        )
        .unwrap();
        let playlist: SubsonicResponse = serde_json::from_str(
            r#"{"subsonic-response":{"status":"ok","playlist":{"song":[{"id":"s1","title":"T","artist":"A"}]}}}"#,
        )
        .unwrap();
        let album: SubsonicResponse = serde_json::from_str(
            r#"{"subsonic-response":{"status":"ok","album":{"song":[{"id":"s2","title":"T2","artist":"A"}]}}}"#,
        )
        .unwrap();
        let top: SubsonicResponse = serde_json::from_str(
            r#"{"subsonic-response":{"status":"ok","topSongs":{"song":[{"id":"s3","title":"T3","artist":"A"}]}}}"#,
        )
        .unwrap();

        assert_eq!(
            playlists
                .subsonic_response
                .getPlaylists
                .unwrap()
                .playlist
                .unwrap()[0]
                .songCount,
            Some(7)
        );
        assert_eq!(
            playlist
                .subsonic_response
                .getPlaylist
                .unwrap()
                .song
                .unwrap()[0]
                .id,
            "s1"
        );
        assert_eq!(
            album.subsonic_response.getAlbum.unwrap().song.unwrap()[0].id,
            "s2"
        );
        assert_eq!(
            top.subsonic_response.getTopSongs.unwrap().song.unwrap()[0].id,
            "s3"
        );
    }

    #[test]
    fn missing_status_becomes_api_error() {
        let envelope: SubsonicResponse =
            serde_json::from_str(r#"{"subsonic-response":{"searchResult3":{"song":[]}}}"#).unwrap();

        match ensure_ok(&envelope.subsonic_response) {
            Err(SubsonicError::Api { message }) => {
                assert!(message.contains("missing status"), "message: {message}");
            }
            other => panic!("expected API error, got {other:?}"),
        }
    }

    #[test]
    fn failed_status_becomes_api_error() {
        let envelope: SubsonicResponse = serde_json::from_str(
            r#"{"subsonic-response":{"status":"failed","error":{"code":40,"message":"wrong password"}}}"#,
        )
        .unwrap();

        match ensure_ok(&envelope.subsonic_response) {
            Err(SubsonicError::Api { message }) => assert_eq!(message, "wrong password"),
            other => panic!("expected API error, got {other:?}"),
        }
    }
}
