//! End-to-end tests for the public Subsonic client using a local mock server.

use std::collections::HashMap;

use alexa_navidrome_bridge::subsonic::SubsonicClient;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

const SEARCH_RESPONSE: &str = r#"{
  "subsonic-response": {
    "status": "ok",
    "version": "1.16.1",
    "type": "navidrome",
    "searchResult3": {
      "song": [
        {"id": "song-42", "title": "Viento", "artist": "Aurora", "album": "Norte", "duration": 213},
        {"id": "song-43", "title": "Lluvia", "artist": "Aurora", "album": "Sur"}
      ],
      "album": [
        {"id": "album-7", "name": "Norte", "artist": "Aurora", "year": 2024}
      ],
      "artist": [
        {"id": "artist-3", "name": "Aurora"}
      ]
    }
  }
}"#;

const PLAYLIST_RESPONSE: &str = r#"{
  "subsonic-response": {
    "status": "ok",
    "version": "1.16.1",
    "playlist": {
      "id": "playlist-1",
      "name": "Favoritas",
      "songCount": 2,
      "song": [
        {"id": "song-42", "title": "Viento", "artist": "Aurora"},
        {"id": "song-43", "title": "Lluvia", "artist": "Aurora"}
      ]
    }
  }
}"#;

const ALBUM_RESPONSE: &str = r#"{
  "subsonic-response": {
    "status": "ok",
    "version": "1.16.1",
    "album": {
      "id": "album-7",
      "name": "Norte",
      "artist": "Aurora",
      "song": [
        {"id": "song-42", "title": "Viento", "artist": "Aurora", "album": "Norte", "duration": 213},
        {"id": "song-44", "title": "Nieve", "artist": "Aurora", "album": "Norte", "duration": 187}
      ]
    }
  }
}"#;

/// Starts a minimal HTTP/1.1 server that maps endpoints to fixed JSON bodies.
async fn start_mock(responses: HashMap<&'static str, &'static str>) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock server");
    let addr = listener.local_addr().expect("mock server address");

    let handle = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut request = Vec::new();
            let mut chunk = [0u8; 1024];
            loop {
                match socket.read(&mut chunk).await {
                    Ok(0) | Err(_) => break,
                    Ok(read) => {
                        request.extend_from_slice(&chunk[..read]);
                        if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                            break;
                        }
                    }
                }
            }

            let request_line = String::from_utf8_lossy(&request)
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned();
            let path = request_line.split_whitespace().nth(1).unwrap_or("/");
            let endpoint = path.split('?').next().unwrap_or(path);
            let body = responses.get(endpoint).copied().unwrap_or("{}");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );

            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });

    (format!("http://{addr}"), handle)
}

fn responses(entries: &[(&'static str, &'static str)]) -> HashMap<&'static str, &'static str> {
    entries.iter().copied().collect()
}

fn client(base: String) -> SubsonicClient {
    SubsonicClient::new(base, "user".to_owned(), "secret".to_owned())
}

#[tokio::test]
async fn search3_parses_songs_albums_and_artists_from_envelope() {
    let (base, server) = start_mock(responses(&[("/rest/search3", SEARCH_RESPONSE)])).await;

    let result = client(base).search3("query").await.expect("search3 result");
    server.abort();

    assert_eq!(result.songs.len(), 2);
    assert_eq!(result.songs[0].id, "song-42");
    assert_eq!(result.songs[0].title, "Viento");
    assert_eq!(result.songs[0].artist, "Aurora");
    assert_eq!(result.songs[0].album.as_deref(), Some("Norte"));
    assert_eq!(result.songs[0].duration, Some(213));
    assert_eq!(result.albums.len(), 1);
    assert_eq!(result.albums[0].name, "Norte");
    assert_eq!(result.artists.len(), 1);
    assert_eq!(result.artists[0].name, "Aurora");
}

#[test]
fn stream_url_contains_signed_auth_parameters_and_mp3_format() {
    let url = client("http://127.0.0.1:1".to_owned()).stream_url("song-42");
    let parsed = reqwest::Url::parse(&url).expect("valid stream URL");
    let pairs: Vec<_> = parsed.query_pairs().collect();

    assert_eq!(parsed.path(), "/rest/stream");
    assert!(pairs
        .iter()
        .any(|(key, value)| key == "id" && value == "song-42"));
    assert!(pairs
        .iter()
        .any(|(key, value)| key == "format" && value == "mp3"));
    assert!(pairs
        .iter()
        .any(|(key, value)| key == "v" && value == "1.16.1"));
    assert!(pairs
        .iter()
        .any(|(key, value)| key == "c" && value == "alexa_navidrome_bridge"));

    let token = pairs
        .iter()
        .find(|(key, _)| key == "t")
        .map(|(_, value)| value.as_ref())
        .expect("auth token");
    assert_eq!(token.len(), 32);
    assert!(token.chars().all(|character| character.is_ascii_hexdigit()));
}

#[tokio::test]
async fn get_playlist_returns_non_empty_songs() {
    let (base, server) = start_mock(responses(&[("/rest/getPlaylist", PLAYLIST_RESPONSE)])).await;

    let songs = client(base)
        .get_playlist("playlist-1")
        .await
        .expect("playlist songs");
    server.abort();

    assert_eq!(songs.len(), 2);
    assert_eq!(songs[0].title, "Viento");
    assert_eq!(songs[1].id, "song-43");
}

#[tokio::test]
async fn get_album_returns_non_empty_songs() {
    let (base, server) = start_mock(responses(&[("/rest/getAlbum", ALBUM_RESPONSE)])).await;

    let songs = client(base)
        .get_album("album-7")
        .await
        .expect("album songs");
    server.abort();

    assert_eq!(songs.len(), 2);
    assert_eq!(songs[0].title, "Viento");
    assert_eq!(songs[1].title, "Nieve");
}
