//! Alexa Skills Kit request/response types and response constructors.

use crate::config::Config;
use crate::state::{peek_queue, set_queue, update_queue, AppState, PlaybackQueue};
use crate::subsonic::{Search3Result, Song, SubsonicClient};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// Incoming Alexa request envelope.
#[derive(Debug, Deserialize)]
pub struct AlexaRequest {
    pub version: String,
    #[serde(default)]
    pub session: Option<Session>,
    #[serde(default)]
    pub context: Option<Context>,
    pub request: RequestBody,
}

impl AlexaRequest {
    /// Returns the Echo device identifier from `context.System.device`, if present.
    pub fn device_id(&self) -> Option<&str> {
        self.context
            .as_ref()?
            .System
            .device
            .as_ref()
            .map(|device| device.deviceId.as_str())
    }

    /// Returns the skill application id, preferring the session over the context.
    pub fn application_id(&self) -> Option<&str> {
        self.session
            .as_ref()
            .and_then(|session| session.application.as_ref())
            .or_else(|| self.context.as_ref()?.System.application.as_ref())
            .map(|application| application.applicationId.as_str())
    }

    /// Returns the intent name when this request is an `IntentRequest`.
    pub fn intent_name(&self) -> Option<&str> {
        match &self.request {
            RequestBody::IntentRequest { intent, .. } => Some(intent.name.as_str()),
            _ => None,
        }
    }
}

/// Discriminated body of an Alexa request.
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum RequestBody {
    #[serde(rename = "LaunchRequest")]
    LaunchRequest,
    #[serde(rename = "IntentRequest")]
    IntentRequest {
        intent: Intent,
        requestId: Option<String>,
    },
    #[serde(rename = "SessionEndedRequest")]
    SessionEndedRequest,
    #[serde(rename = "AudioPlayer.PlaybackStarted")]
    PlaybackStarted(AudioPlayerEvent),
    #[serde(rename = "AudioPlayer.PlaybackNearlyFinished")]
    PlaybackNearlyFinished(AudioPlayerEvent),
    #[serde(rename = "AudioPlayer.PlaybackStopped")]
    PlaybackStopped(AudioPlayerEvent),
    #[serde(rename = "AudioPlayer.PlaybackFinished")]
    PlaybackFinished(AudioPlayerEvent),
    #[serde(rename = "AudioPlayer.PlaybackFailed")]
    PlaybackFailed(AudioPlayerEvent),
}

/// An intent together with its slot values.
#[derive(Debug, Deserialize)]
pub struct Intent {
    pub name: String,
    #[serde(default)]
    pub slots: HashMap<String, Slot>,
}

/// A single intent slot.
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct Slot {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
}

/// Session metadata attached to most non-audio requests.
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct Session {
    #[serde(default)]
    pub sessionId: Option<String>,
    #[serde(default)]
    pub application: Option<Application>,
    #[serde(default)]
    pub user: Option<User>,
}

/// Skill application metadata.
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct Application {
    pub applicationId: String,
}

/// Alexa user metadata.
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct User {
    #[serde(default)]
    pub userId: Option<String>,
}

/// Context object of an Alexa request.
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct Context {
    pub System: SystemContext,
}

/// The `System` part of the request context.
#[derive(Debug, Deserialize)]
pub struct SystemContext {
    #[serde(default)]
    pub device: Option<Device>,
    #[serde(default)]
    pub application: Option<Application>,
}

/// Device metadata from the request context.
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct Device {
    pub deviceId: String,
}

/// Payload common to all `AudioPlayer.*` requests.
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct AudioPlayerEvent {
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub offsetInMilliseconds: Option<i64>,
    #[serde(default)]
    pub error: Option<serde_json::Value>,
}

/// Outgoing Alexa response envelope.
#[derive(Debug, Serialize)]
pub struct AlexaResponse {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ResponseBody>,
}

/// Body of an outgoing Alexa response.
#[allow(non_snake_case)]
#[derive(Debug, Serialize, Default)]
pub struct ResponseBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputSpeech: Option<OutputSpeech>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reprompt: Option<Reprompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shouldEndSession: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub directives: Vec<Directive>,
}

/// Plain-text speech output.
#[derive(Debug, Serialize)]
pub struct OutputSpeech {
    #[serde(rename = "type")]
    pub speech_type: String,
    pub text: String,
}

/// Reprompt speech for an open session.
#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
pub struct Reprompt {
    pub outputSpeech: OutputSpeech,
}

/// An AudioPlayer directive.
#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
pub struct Directive {
    #[serde(rename = "type")]
    pub directive_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playBehavior: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audioItem: Option<AudioItem>,
}

/// Audio item carried by a play directive.
#[derive(Debug, Serialize)]
pub struct AudioItem {
    pub stream: Stream,
}

/// Stream description of an audio item.
#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
pub struct Stream {
    pub url: String,
    pub token: String,
    pub offsetInMilliseconds: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expectedPreviousToken: Option<String>,
}

/// Builds a response that speaks `text` and optionally ends the session.
pub fn speak(text: &str, end_session: bool) -> AlexaResponse {
    AlexaResponse {
        version: "1.0".to_owned(),
        response: Some(ResponseBody {
            outputSpeech: Some(OutputSpeech {
                speech_type: "PlainText".to_owned(),
                text: text.to_owned(),
            }),
            shouldEndSession: Some(end_session),
            ..ResponseBody::default()
        }),
    }
}

/// Builds a response with an `AudioPlayer.Play` directive that replaces the queue.
pub fn audio_play(url: &str, token: &str, offset_ms: i64, behavior: &str) -> AlexaResponse {
    AlexaResponse {
        version: "1.0".to_owned(),
        response: Some(ResponseBody {
            directives: vec![Directive {
                directive_type: "AudioPlayer.Play".to_owned(),
                playBehavior: Some(behavior.to_owned()),
                audioItem: Some(AudioItem {
                    stream: Stream {
                        url: url.to_owned(),
                        token: token.to_owned(),
                        offsetInMilliseconds: offset_ms,
                        expectedPreviousToken: None,
                    },
                }),
            }],
            ..ResponseBody::default()
        }),
    }
}

/// Builds a response that enqueues the next track after `expected_prev`.
pub fn audio_play_enqueue(
    url: &str,
    token: &str,
    offset_ms: i64,
    expected_prev: &str,
) -> AlexaResponse {
    AlexaResponse {
        version: "1.0".to_owned(),
        response: Some(ResponseBody {
            directives: vec![Directive {
                directive_type: "AudioPlayer.Play".to_owned(),
                playBehavior: Some("ENQUEUE".to_owned()),
                audioItem: Some(AudioItem {
                    stream: Stream {
                        url: url.to_owned(),
                        token: token.to_owned(),
                        offsetInMilliseconds: offset_ms,
                        expectedPreviousToken: Some(expected_prev.to_owned()),
                    },
                }),
            }],
            ..ResponseBody::default()
        }),
    }
}

/// Builds a response with an `AudioPlayer.Stop` directive.
pub fn audio_stop() -> AlexaResponse {
    AlexaResponse {
        version: "1.0".to_owned(),
        response: Some(ResponseBody {
            directives: vec![Directive {
                directive_type: "AudioPlayer.Stop".to_owned(),
                playBehavior: None,
                audioItem: None,
            }],
            shouldEndSession: Some(true),
            ..ResponseBody::default()
        }),
    }
}

/// Builds a minimal empty response.
pub fn empty() -> AlexaResponse {
    AlexaResponse {
        version: "1.0".to_owned(),
        response: Some(ResponseBody::default()),
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Returns a non-empty slot value, matching the slot key case-insensitively.
fn slot_value<'a>(slots: &'a HashMap<String, Slot>, name: &str) -> Option<&'a str> {
    slots
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .and_then(|(_, slot)| {
            debug!(slot = %slot.name, "matching intent slot");
            slot.value.as_deref()
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// Returns the request's device ID or a stable fallback for devices that omit it.
fn device_id_or_default(req: &AlexaRequest) -> &str {
    req.device_id().unwrap_or("default")
}

/// Deterministic audioPlayer token for a Subsonic song ID.
fn token_for(song_id: &str) -> String {
    format!("track-{song_id}")
}

/// Wraps text in the `PlainText` output speech shape.
fn speech_text(text: &str) -> OutputSpeech {
    OutputSpeech {
        speech_type: "PlainText".to_owned(),
        text: text.to_owned(),
    }
}

/// Builds a play directive response that can also carry output speech.
fn build_play_response(
    url: &str,
    token: &str,
    offset_ms: i64,
    behavior: &str,
    speech: Option<&str>,
) -> AlexaResponse {
    let mut response = audio_play(url, token, offset_ms, behavior);
    if let Some(body) = response.response.as_mut() {
        body.outputSpeech = speech.map(speech_text);
    }
    response
}

/// Builds a stop directive response that can also carry output speech.
fn build_stop_response(speech: Option<&str>) -> AlexaResponse {
    AlexaResponse {
        version: "1.0".to_owned(),
        response: Some(ResponseBody {
            outputSpeech: speech.map(speech_text),
            shouldEndSession: Some(true),
            directives: vec![Directive {
                directive_type: "AudioPlayer.Stop".to_owned(),
                playBehavior: None,
                audioItem: None,
            }],
            ..ResponseBody::default()
        }),
    }
}

/// Stores `tracks` as the device queue and starts playback of the first one.
fn start_queue(
    req: &AlexaRequest,
    tracks: &[Song],
    client: &SubsonicClient,
    state: &AppState,
    confirmation: impl FnOnce(&Song) -> String,
    empty_message: &str,
) -> AlexaResponse {
    let device = device_id_or_default(req);
    let Some(first) = tracks.first() else {
        warn!(device, "no tracks available for playback");
        return speak(empty_message, true);
    };

    let ids = tracks.iter().map(|song| song.id.clone()).collect();
    let token = token_for(&first.id);
    let mut queue = PlaybackQueue::new(ids);
    queue.save_offset(token.clone(), 0);
    let replacement = queue.clone();
    if update_queue(state, device, |current| *current = replacement).is_none() {
        // No queue existed for this device yet.
        set_queue(state, device, queue);
    }

    let url = client.stream_url(&first.id);
    let speech = confirmation(first);
    info!(
        device,
        track_id = %first.id,
        album = ?first.album,
        duration_secs = ?first.duration,
        "starting playback"
    );
    build_play_response(&url, &token, 0, "REPLACE_ALL", Some(&speech))
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/// Dispatches an Alexa request to the matching intent or AudioPlayer handler.
///
/// Skill-ID enforcement for a real 401 response belongs to the webhook layer;
/// this function only logs the mismatch and returns an empty response.
pub async fn handle_request(
    req: &AlexaRequest,
    client: &SubsonicClient,
    state: &AppState,
    config: &Config,
) -> AlexaResponse {
    match config.alexa_skill_id.as_deref() {
        Some(expected) if req.application_id() != Some(expected) => {
            warn!(
                expected,
                actual = ?req.application_id(),
                "unexpected Alexa skill ID; webhook must reject this request"
            );
            return empty();
        }
        _ => {}
    }

    if let RequestBody::IntentRequest { requestId, .. } = &req.request {
        debug!(request_id = ?requestId, "intent request metadata");
    }
    if let Some(session) = &req.session {
        debug!(
            session_id = ?session.sessionId,
            user_id = ?session.user.as_ref().and_then(|user| user.userId.as_deref()),
            "session metadata"
        );
    }

    info!(version = %req.version, intent = ?req.intent_name(), "dispatching Alexa request");
    match &req.request {
        RequestBody::LaunchRequest => handle_launch(),
        RequestBody::SessionEndedRequest => handle_session_ended(),
        RequestBody::IntentRequest { intent, .. } => match intent.name.as_str() {
            "PlayMusicIntent" => handle_play_music(req, &intent.slots, client, state).await,
            "PlayPlaylistIntent" => handle_play_playlist(req, &intent.slots, client, state).await,
            "AMAZON.NextIntent" => handle_next(req, state, client).await,
            "AMAZON.PreviousIntent" => handle_previous(req, state, client).await,
            "AMAZON.PauseIntent" | "AMAZON.StopIntent" | "AMAZON.CancelIntent" => {
                handle_pause_stop_cancel()
            }
            "AMAZON.ResumeIntent" => handle_resume(req, state, client).await,
            "AMAZON.NavigateHomeIntent" => handle_navigate_home(),
            "AMAZON.ShuffleOnIntent" => handle_shuffle(req, state, true).await,
            "AMAZON.ShuffleOffIntent" => handle_shuffle(req, state, false).await,
            other => {
                warn!(intent = other, "unsupported intent");
                speak("No he reconocido esa solicitud.", true)
            }
        },
        RequestBody::PlaybackNearlyFinished(_) => {
            handle_playback_nearly_finished(req, state, client).await
        }
        RequestBody::PlaybackStopped(_) => handle_playback_stopped(req, state),
        RequestBody::PlaybackStarted(_) => handle_playback_started(req, state),
        RequestBody::PlaybackFinished(_) | RequestBody::PlaybackFailed(_) => {
            handle_playback_started_finished_failed(req)
        }
    }
}

// ---------------------------------------------------------------------------
// Intent handlers
// ---------------------------------------------------------------------------

/// Welcomes the user and leaves the session open for a voice command.
pub fn handle_launch() -> AlexaResponse {
    info!("handling LaunchRequest");
    speak(
        "Bienvenido a tu biblioteca musical. Dime qué canción, artista o álbum quieres escuchar.",
        false,
    )
}

/// Returns to the skill home state, stopping playback and ending the session.
pub fn handle_navigate_home() -> AlexaResponse {
    info!("handling AMAZON.NavigateHomeIntent");
    AlexaResponse {
        version: "1.0".to_owned(),
        response: Some(ResponseBody {
            outputSpeech: Some(OutputSpeech {
                speech_type: "PlainText".to_owned(),
                text: "Volviendo al inicio.".to_owned(),
            }),
            shouldEndSession: Some(true),
            directives: vec![Directive {
                directive_type: "AudioPlayer.Stop".to_owned(),
                playBehavior: None,
                audioItem: None,
            }],
            ..ResponseBody::default()
        }),
    }
}

/// Removes common Spanish prefixes from a search query before sending to Navidrome.
/// Tries longer prefixes first; if the result is empty, returns the original query.
/// Strips all matching prefixes iteratively, respecting the order (longest first).
pub fn sanitize_search_query(query: &str) -> String {
    // Prefixes ordered by length descending to try longest first
    let prefixes: &[&str] = &[
        "reproduce musica de ",
        "pon musica de ",
        "musica de ",
        "música de ",
        "reproduce ",
        "pon ",
    ];
    let mut q = query;
    loop {
        let mut stripped = false;
        for prefix in prefixes {
            if let Some(rest) = q.strip_prefix(prefix) {
                q = rest;
                stripped = true;
                break;
            }
        }
        if !stripped {
            break;
        }
    }
    if q.is_empty() {
        query.to_owned()
    } else {
        q.to_string()
    }
}

/// Searches songs, albums and artists (in that priority order) and plays the first track.
pub async fn handle_play_music(
    req: &AlexaRequest,
    slots: &HashMap<String, Slot>,
    client: &SubsonicClient,
    state: &AppState,
) -> AlexaResponse {
    let Some(query) = slot_value(slots, "Query").or_else(|| slot_value(slots, "SearchQuery"))
    else {
        warn!("PlayMusicIntent received no query slot");
        return speak(
            "No he entendido qué quieres escuchar. Dime el nombre de una canción, artista o álbum.",
            true,
        );
    };

    let search_query = sanitize_search_query(query);
    let results = match client.search3(&search_query).await {
        Ok(results) => results,
        Err(err) => {
            error!(error = %err, query, "search3 failed");
            return speak(
                "No he podido buscar en tu biblioteca. Inténtalo de nuevo.",
                true,
            );
        }
    };

    let Search3Result {
        songs,
        albums,
        artists,
    } = results;

    let tracks = if !songs.is_empty() {
        songs
    } else if let Some(album) = albums.first() {
        info!(
            album_id = %album.id,
            album = %album.name,
            artist = ?album.artist,
            expanded_songs = album.song.as_ref().map_or(0, Vec::len),
            "loading album tracks"
        );
        match client.get_album(&album.id).await {
            Ok(tracks) => tracks,
            Err(err) => {
                error!(error = %err, album_id = %album.id, "getAlbum failed");
                return speak(
                    "No he podido buscar en tu biblioteca. Inténtalo de nuevo.",
                    true,
                );
            }
        }
    } else if let Some(artist) = artists.first() {
        info!(artist_id = %artist.id, artist = %artist.name, "loading top songs");
        match client.get_artist_top_songs(&artist.id).await {
            Ok(tracks) => tracks,
            Err(err) => {
                error!(error = %err, artist_id = %artist.id, "getTopSongs failed");
                return speak(
                    "No he podido buscar en tu biblioteca. Inténtalo de nuevo.",
                    true,
                );
            }
        }
    } else {
        Vec::new()
    };

    start_queue(
        req,
        &tracks,
        client,
        state,
        |song| {
            let title = song.title.as_str();
            let artist = song.artist.as_str();
            format!("Reproduciendo {title} de {artist}.")
        },
        "No he encontrado resultados para tu búsqueda.",
    )
}

/// Finds a playlist by (case-insensitive) name fragment and plays its songs.
pub async fn handle_play_playlist(
    req: &AlexaRequest,
    slots: &HashMap<String, Slot>,
    client: &SubsonicClient,
    state: &AppState,
) -> AlexaResponse {
    let Some(query) = slot_value(slots, "Playlist") else {
        warn!("PlayPlaylistIntent received no Playlist slot");
        return speak("No he entendido qué lista quieres escuchar.", true);
    };

    let playlists = match client.get_playlists().await {
        Ok(playlists) => playlists,
        Err(err) => {
            error!(error = %err, "getPlaylists failed");
            return speak(
                "No he podido consultar tus listas de reproducción. Inténtalo de nuevo.",
                true,
            );
        }
    };

    let needle = query.to_lowercase();
    let Some(playlist) = playlists
        .iter()
        .find(|playlist| playlist.name.to_lowercase().contains(&needle))
    else {
        warn!(query, "no playlist matched the spoken name");
        return speak("No he encontrado esa lista de reproducción.", true);
    };
    info!(
        playlist_id = %playlist.id,
        playlist = %playlist.name,
        song_count = ?playlist.songCount,
        "loading playlist tracks"
    );

    let tracks = match client.get_playlist(&playlist.id).await {
        Ok(tracks) => tracks,
        Err(err) => {
            error!(error = %err, playlist_id = %playlist.id, "getPlaylist failed");
            return speak(
                "No he podido cargar la lista de reproducción. Inténtalo de nuevo.",
                true,
            );
        }
    };

    start_queue(
        req,
        &tracks,
        client,
        state,
        |_song| {
            let name = playlist.name.as_str();
            format!("Reproduciendo la lista {name}.")
        },
        "La lista de reproducción está vacía.",
    )
}

/// Advances the device queue and plays the next track, or stops at the end.
pub async fn handle_next(
    req: &AlexaRequest,
    state: &AppState,
    client: &SubsonicClient,
) -> AlexaResponse {
    let device = device_id_or_default(req);
    let mut advanced = false;
    let queue = update_queue(state, device, |queue| {
        if let Some(track_id) = queue.advance() {
            let token = token_for(&track_id);
            queue.save_offset(token, 0);
            advanced = true;
        }
    });
    let Some(queue) = queue.filter(|_| advanced) else {
        warn!(device, "no next track in queue");
        return build_stop_response(Some("No hay más canciones en la cola."));
    };
    let Some(track_id) = queue.current_track() else {
        warn!(device, "no current track after advancing");
        return build_stop_response(Some("No hay más canciones en la cola."));
    };

    let token = token_for(track_id);
    let url = client.stream_url(track_id);
    info!(device, track_id = %track_id, "playing next track");
    build_play_response(&url, &token, 0, "REPLACE_ALL", None)
}

/// Moves back in the device queue and plays the previous track.
pub async fn handle_previous(
    req: &AlexaRequest,
    state: &AppState,
    client: &SubsonicClient,
) -> AlexaResponse {
    let device = device_id_or_default(req);
    let mut moved = false;
    let queue = update_queue(state, device, |queue| {
        if let Some(track_id) = queue.retreat() {
            let token = token_for(&track_id);
            queue.save_offset(token, 0);
            moved = true;
        }
    });
    let Some(queue) = queue.filter(|_| moved) else {
        warn!(device, "no previous track in queue");
        return speak("Ya estás en la primera canción.", true);
    };
    let Some(track_id) = queue.current_track() else {
        warn!(device, "no current track after retreating");
        return speak("Ya estás en la primera canción.", true);
    };

    let token = token_for(track_id);
    let url = client.stream_url(track_id);
    info!(device, track_id = %track_id, "playing previous track");
    build_play_response(&url, &token, 0, "REPLACE_ALL", None)
}

/// Stops AudioPlayer playback for Pause, Stop and Cancel intents.
pub fn handle_pause_stop_cancel() -> AlexaResponse {
    info!("stopping playback");
    audio_stop()
}

/// Resumes the current track from its saved token and offset.
pub async fn handle_resume(
    req: &AlexaRequest,
    state: &AppState,
    client: &SubsonicClient,
) -> AlexaResponse {
    let device = device_id_or_default(req);
    let queue = peek_queue(state, device);
    let Some(token) = queue.token.clone() else {
        warn!(device, "nothing to resume");
        return speak("No hay nada para reanudar.", true);
    };
    // The saved token identifies the track the offset belongs to; never use a
    // stale `index` that may point at a different song.
    let Some(position) = queue.position_of_token(&token) else {
        warn!(device, token = %token, "saved token is not in the queue");
        return speak("No hay nada para reanudar.", true);
    };
    let queue = update_queue(state, device, |queue| queue.index = position).unwrap_or(queue);
    let Some(track_id) = queue.current_track() else {
        warn!(device, "nothing to resume");
        return speak("No hay nada para reanudar.", true);
    };
    let offset_ms = queue.offset_ms;

    let url = client.stream_url(track_id);
    info!(device, track_id, offset_ms, "resuming playback");
    build_play_response(&url, &token, offset_ms, "REPLACE_ALL", None)
}

/// Enables or disables random ordering of the unplayed part of the queue.
pub async fn handle_shuffle(req: &AlexaRequest, state: &AppState, on: bool) -> AlexaResponse {
    let device = device_id_or_default(req);
    update_queue(state, device, |queue| {
        if on {
            queue.shuffle_remaining();
        } else {
            queue.restore_order();
        }
    });

    info!(device, shuffled = on, "updated queue ordering");
    if on {
        speak("Orden aleatorio activado.", true)
    } else {
        speak("Orden aleatorio desactivado.", true)
    }
}

// ---------------------------------------------------------------------------
// AudioPlayer event handlers
// ---------------------------------------------------------------------------

/// Enqueues the next track so playback continues without a gap.
pub async fn handle_playback_nearly_finished(
    req: &AlexaRequest,
    state: &AppState,
    client: &SubsonicClient,
) -> AlexaResponse {
    let RequestBody::PlaybackNearlyFinished(event) = &req.request else {
        warn!("nearly-finished handler called with the wrong request type");
        return empty();
    };

    let device = device_id_or_default(req);
    let queue = peek_queue(state, device);
    // Alexa's token identifies which track just finished. Fall back to the
    // stored index only when the event has no recognizable token.
    let position = event
        .token
        .as_deref()
        .and_then(|token| queue.position_of_token(token))
        .unwrap_or(queue.index);
    let Some(current_id) = queue.tracks.get(position) else {
        warn!(
            device,
            position, "event token does not match a queued track"
        );
        return empty();
    };
    let Some(next_id) = (if position == queue.index {
        queue.next_track()
    } else {
        position
            .checked_add(1)
            .and_then(|next| queue.tracks.get(next))
            .cloned()
    }) else {
        info!(device, "no next track to enqueue");
        return empty();
    };
    // Only reuse the event token when it really belongs to the current track;
    // otherwise derive it from the queue so ENQUEUE is accepted by Alexa.
    let current_token = event
        .token
        .as_deref()
        .filter(|token| queue.position_of_token(token) == Some(position))
        .map(str::to_owned)
        .unwrap_or_else(|| token_for(current_id));

    let token = token_for(&next_id);
    let url = client.stream_url(&next_id);
    info!(device, next_id = %next_id, "enqueuing next track");
    audio_play_enqueue(&url, &token, 0, &current_token)
}

/// Persists the token and offset reported when playback stops.
pub fn handle_playback_stopped(req: &AlexaRequest, state: &AppState) -> AlexaResponse {
    let RequestBody::PlaybackStopped(event) = &req.request else {
        warn!("stopped handler called with the wrong request type");
        return empty();
    };
    let Some(token) = event.token.as_deref() else {
        warn!("PlaybackStopped event has no token");
        return empty();
    };
    let offset_ms = event.offsetInMilliseconds.unwrap_or_default();
    let device = device_id_or_default(req);
    if update_queue(state, device, |queue| {
        if let Some(position) = queue.position_of_token(token) {
            queue.index = position;
        }
        queue.save_offset(token.to_owned(), offset_ms);
    })
    .is_none()
    {
        // Keep the previous behavior for events arriving before any queue was
        // created, without doing a read-modify-write under the lock.
        let mut queue = PlaybackQueue::new(Vec::new());
        queue.save_offset(token.to_owned(), offset_ms);
        set_queue(state, device, queue);
    }

    info!(device, token, offset_ms, "saved playback position");
    empty()
}

/// Applies an `AudioPlayer.PlaybackStarted` event to the device queue.
///
/// The event token identifies the track Alexa actually started, so it takes
/// precedence over a queue index that may still refer to the previous track.
pub fn handle_playback_started(req: &AlexaRequest, state: &AppState) -> AlexaResponse {
    let RequestBody::PlaybackStarted(event) = &req.request else {
        warn!("playback-started handler called with the wrong request type");
        return empty();
    };
    info!(
        token = ?event.token,
        offset = ?event.offsetInMilliseconds,
        error = ?event.error,
        "playback started"
    );

    let device = device_id_or_default(req);
    if let Some(token) = event.token.as_deref() {
        let updated = update_queue(state, device, |queue| {
            if let Some(position) = queue.position_of_token(token) {
                queue.index = position;
            }
        });
        if updated.is_none() {
            warn!(device, token, "no queue to apply playback start to");
        }
    }
    empty()
}

/// Logs informational AudioPlayer events without returning directives or speech.
pub fn handle_playback_started_finished_failed(req: &AlexaRequest) -> AlexaResponse {
    match &req.request {
        RequestBody::PlaybackStarted(event) => {
            info!(
                token = ?event.token,
                offset = ?event.offsetInMilliseconds,
                error = ?event.error,
                "playback started"
            );
        }
        RequestBody::PlaybackFinished(event) => {
            info!(
                token = ?event.token,
                offset = ?event.offsetInMilliseconds,
                error = ?event.error,
                "playback finished"
            );
        }
        RequestBody::PlaybackFailed(event) => {
            warn!(
                token = ?event.token,
                offset = ?event.offsetInMilliseconds,
                error = ?event.error,
                "playback failed"
            );
        }
        _ => warn!("audio event handler called with the wrong request type"),
    }
    empty()
}

/// Acknowledges a finished Alexa session with an empty 200 response.
pub fn handle_session_ended() -> AlexaResponse {
    info!("session ended");
    empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::sync::{Arc, RwLock};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn launch_payload() -> Value {
        json!({
            "version": "1.0",
            "session": {
                "sessionId": "SessionId.abc",
                "application": { "applicationId": "amzn1.ask.skill.123" },
                "user": { "userId": "amzn1.ask.account.456" }
            },
            "context": {
                "System": {
                    "application": { "applicationId": "amzn1.ask.skill.123" },
                    "device": { "deviceId": "amzn1.ask.device.789" }
                }
            },
            "request": {
                "type": "LaunchRequest",
                "requestId": "amzn1.echo-api.request.1",
                "timestamp": "2026-01-01T00:00:00Z",
                "locale": "es-ES"
            }
        })
    }

    fn intent_payload() -> Value {
        json!({
            "version": "1.0",
            "session": {
                "sessionId": "SessionId.abc",
                "application": { "applicationId": "amzn1.ask.skill.123" },
                "user": { "userId": "amzn1.ask.account.456" }
            },
            "context": {
                "System": {
                    "device": { "deviceId": "amzn1.ask.device.789" }
                }
            },
            "request": {
                "type": "IntentRequest",
                "requestId": "amzn1.echo-api.request.2",
                "intent": {
                    "name": "PlayMusicIntent",
                    "slots": { "Query": { "name": "Query", "value": "queen" } }
                }
            }
        })
    }

    #[test]
    fn deserializes_launch_request() {
        let request: AlexaRequest = serde_json::from_value(launch_payload()).unwrap();

        assert_eq!(request.version, "1.0");
        assert!(matches!(request.request, RequestBody::LaunchRequest));
    }

    #[test]
    fn deserializes_intent_request_with_query_slot() {
        let request: AlexaRequest = serde_json::from_value(intent_payload()).unwrap();

        match &request.request {
            RequestBody::IntentRequest { intent, requestId } => {
                assert_eq!(intent.name, "PlayMusicIntent");
                assert_eq!(intent.slots["Query"].value.as_deref(), Some("queen"));
                assert_eq!(requestId.as_deref(), Some("amzn1.echo-api.request.2"));
            }
            other => panic!("unexpected request variant: {other:?}"),
        }
    }

    #[test]
    fn deserializes_playback_nearly_finished() {
        let payload = json!({
            "version": "1.0",
            "context": {
                "System": { "device": { "deviceId": "amzn1.ask.device.789" } }
            },
            "request": {
                "type": "AudioPlayer.PlaybackNearlyFinished",
                "token": "track-42",
                "offsetInMilliseconds": 12345
            }
        });
        let request: AlexaRequest = serde_json::from_value(payload).unwrap();

        match &request.request {
            RequestBody::PlaybackNearlyFinished(event) => {
                assert_eq!(event.token.as_deref(), Some("track-42"));
                assert_eq!(event.offsetInMilliseconds, Some(12_345));
                assert!(event.error.is_none());
            }
            other => panic!("unexpected request variant: {other:?}"),
        }
        assert!(request.session.is_none());
    }

    #[test]
    fn helper_methods_extract_device_application_and_intent() {
        let request: AlexaRequest = serde_json::from_value(intent_payload()).unwrap();

        assert_eq!(request.device_id(), Some("amzn1.ask.device.789"));
        assert_eq!(request.application_id(), Some("amzn1.ask.skill.123"));
        assert_eq!(request.intent_name(), Some("PlayMusicIntent"));
    }

    #[test]
    fn application_id_falls_back_to_context_and_handles_missing_data() {
        let mut payload = intent_payload();
        payload["session"]["application"] = Value::Null;
        let request: AlexaRequest = serde_json::from_value(payload.clone()).unwrap();
        assert_eq!(request.application_id(), None);

        payload["context"] = json!({
            "System": { "application": { "applicationId": "amzn1.ask.skill.from-context" } }
        });
        let request: AlexaRequest = serde_json::from_value(payload).unwrap();
        assert_eq!(
            request.application_id(),
            Some("amzn1.ask.skill.from-context")
        );
        assert_eq!(request.device_id(), None);

        let launch: AlexaRequest = serde_json::from_value(launch_payload()).unwrap();
        assert_eq!(launch.intent_name(), None);
    }

    #[test]
    fn speak_serializes_plain_text_output() {
        let value = serde_json::to_value(speak("Hola", false)).unwrap();

        assert_eq!(value["version"], "1.0");
        assert_eq!(value["response"]["outputSpeech"]["type"], "PlainText");
        assert_eq!(value["response"]["outputSpeech"]["text"], "Hola");
        assert_eq!(value["response"]["shouldEndSession"], false);
        assert!(value["response"]["directives"].is_null());
    }

    #[test]
    fn audio_play_serializes_play_directive() {
        let value = serde_json::to_value(audio_play(
            "https://example.org/track.mp3",
            "track-42",
            500,
            "REPLACE_ALL",
        ))
        .unwrap();
        let directive = &value["response"]["directives"][0];

        assert_eq!(directive["type"], "AudioPlayer.Play");
        assert_eq!(directive["playBehavior"], "REPLACE_ALL");
        assert_eq!(
            directive["audioItem"]["stream"]["url"],
            "https://example.org/track.mp3"
        );
        assert_eq!(directive["audioItem"]["stream"]["token"], "track-42");
        assert_eq!(
            directive["audioItem"]["stream"]["offsetInMilliseconds"],
            500
        );
        assert!(directive["audioItem"]["stream"]["expectedPreviousToken"].is_null());
        assert!(value["response"]["shouldEndSession"].is_null());
        assert!(value["response"]["outputSpeech"].is_null());
    }

    #[test]
    fn audio_play_enqueue_serializes_expected_previous_token() {
        let value = serde_json::to_value(audio_play_enqueue(
            "https://example.org/next.mp3",
            "track-43",
            0,
            "track-42",
        ))
        .unwrap();
        let directive = &value["response"]["directives"][0];

        assert_eq!(directive["playBehavior"], "ENQUEUE");
        assert_eq!(
            directive["audioItem"]["stream"]["expectedPreviousToken"],
            "track-42"
        );
        assert!(value["response"]["outputSpeech"].is_null());
        assert!(value["response"]["shouldEndSession"].is_null());
    }

    #[test]
    fn audio_stop_serializes_stop_directive() {
        let value = serde_json::to_value(audio_stop()).unwrap();

        assert_eq!(
            value["response"]["directives"][0]["type"],
            "AudioPlayer.Stop"
        );
        assert_eq!(value["response"]["shouldEndSession"], true);
        assert!(value["response"]["outputSpeech"].is_null());
        let directive = &value["response"]["directives"][0];
        assert!(directive.get("playBehavior").is_none());
        assert!(directive.get("audioItem").is_none());
    }

    #[test]
    fn empty_serializes_empty_response_body() {
        let value = serde_json::to_value(empty()).unwrap();

        assert_eq!(value, json!({ "version": "1.0", "response": {} }));
    }

    // --- Handler tests with a local mock Subsonic server -------------------

    struct MockServer {
        base: String,
        task: tokio::task::JoinHandle<()>,
    }

    impl MockServer {
        /// Starts a minimal HTTP server answering one request per route entry.
        async fn start(responses: Vec<(&'static str, Value)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind mock Subsonic server");
            let addr = listener.local_addr().expect("mock server address");

            let task = tokio::spawn(async move {
                for (method, body) in responses {
                    let Ok((mut socket, _)) = listener.accept().await else {
                        break;
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

                    let request_line = String::from_utf8_lossy(&request)
                        .lines()
                        .next()
                        .unwrap_or_default()
                        .to_owned();
                    let called_method = request_line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|target| target.split('?').next())
                        .and_then(|path| path.rsplit('/').next());
                    let payload = if called_method == Some(method) {
                        body
                    } else {
                        json!({
                            "subsonic-response": {
                                "status": "failed",
                                "error": { "message": format!("unexpected request: {request_line}") }
                            }
                        })
                    };

                    let bytes = payload.to_string();
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        bytes.len(),
                        bytes
                    );
                    if socket.write_all(response.as_bytes()).await.is_err() {
                        break;
                    }
                    let _ = socket.shutdown().await;
                }
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

    impl Drop for MockServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    fn test_state() -> AppState {
        Arc::new(RwLock::new(HashMap::new()))
    }

    fn test_config() -> Config {
        Config {
            navidrome_url: "http://127.0.0.1:1".into(),
            navidrome_user: "user".into(),
            navidrome_password: "password".into(),
            alexa_skill_id: None,
            port: 3000,
            skip_signature_verification: true,
        }
    }

    fn device_payload(request: Value) -> Value {
        json!({
            "version": "1.0",
            "context": { "System": { "device": { "deviceId": "device-1" } } },
            "request": request
        })
    }

    fn intent_payload_named(name: &str, slots: Value) -> Value {
        device_payload(json!({
            "type": "IntentRequest",
            "intent": { "name": name, "slots": slots }
        }))
    }

    fn parse(payload: Value) -> AlexaRequest {
        serde_json::from_value(payload).expect("valid Alexa request payload")
    }

    #[test]
    fn handle_launch_returns_welcome_speech() {
        let value = serde_json::to_value(handle_launch()).unwrap();

        assert!(value["response"]["outputSpeech"]["text"]
            .as_str()
            .unwrap()
            .contains("Bienvenido"));
        assert_eq!(value["response"]["shouldEndSession"], false);
    }

    #[tokio::test]
    async fn handle_request_navigate_home_speaks_and_stops_playback() {
        let server = MockServer::start(Vec::new()).await;
        let request = parse(intent_payload_named("AMAZON.NavigateHomeIntent", json!({})));

        let value = serde_json::to_value(
            handle_request(&request, &server.client(), &test_state(), &test_config()).await,
        )
        .unwrap();

        assert!(value["response"]["outputSpeech"]["text"]
            .as_str()
            .unwrap()
            .contains("Volviendo al inicio"));
        let directives = value["response"]["directives"].as_array().unwrap();
        assert!(directives
            .iter()
            .any(|directive| directive["type"] == "AudioPlayer.Stop"));
        assert_eq!(value["response"]["shouldEndSession"], true);
    }

    #[tokio::test]
    async fn handle_next_advances_queue_and_returns_play_directive() {
        let server = MockServer::start(Vec::new()).await;
        let state = test_state();
        set_queue(
            &state,
            "device-1",
            PlaybackQueue::new(vec!["s1".into(), "s2".into(), "s3".into()]),
        );
        let request = parse(intent_payload_named("AMAZON.NextIntent", json!({})));

        let response = handle_next(&request, &state, &server.client()).await;
        let value = serde_json::to_value(response).unwrap();
        let directive = &value["response"]["directives"][0];

        assert_eq!(directive["type"], "AudioPlayer.Play");
        assert_eq!(directive["playBehavior"], "REPLACE_ALL");
        assert_eq!(directive["audioItem"]["stream"]["token"], "track-s2");
        let queue = peek_queue(&state, "device-1");
        assert_eq!(queue.current_track(), Some("s2"));
        assert_eq!(queue.token.as_deref(), Some("track-s2"));
    }

    #[tokio::test]
    async fn handle_playback_nearly_finished_enqueues_next_track() {
        let server = MockServer::start(Vec::new()).await;
        let state = test_state();
        let mut queue = PlaybackQueue::new(vec!["s1".into(), "s2".into()]);
        queue.save_offset("track-s1".into(), 1_000);
        set_queue(&state, "device-1", queue);
        let request = parse(device_payload(json!({
            "type": "AudioPlayer.PlaybackNearlyFinished",
            "token": "track-s1",
            "offsetInMilliseconds": 1000
        })));

        let response = handle_playback_nearly_finished(&request, &state, &server.client()).await;
        let value = serde_json::to_value(response).unwrap();
        let directive = &value["response"]["directives"][0];

        assert_eq!(directive["playBehavior"], "ENQUEUE");
        assert_eq!(directive["audioItem"]["stream"]["token"], "track-s2");
        assert_eq!(
            directive["audioItem"]["stream"]["expectedPreviousToken"],
            "track-s1"
        );
        assert!(value["response"]["outputSpeech"].is_null());
        assert_eq!(peek_queue(&state, "device-1").index, 0);
    }

    #[tokio::test]
    async fn natural_queue_advance_uses_event_token_for_successive_enqueues() {
        let server = MockServer::start(Vec::new()).await;
        let state = test_state();
        set_queue(
            &state,
            "device-1",
            PlaybackQueue::new(vec!["s1".into(), "s2".into(), "s3".into()]),
        );

        let first_request = parse(device_payload(json!({
            "type": "AudioPlayer.PlaybackNearlyFinished",
            "token": "track-s1",
            "offsetInMilliseconds": 1000
        })));
        let first = serde_json::to_value(
            handle_playback_nearly_finished(&first_request, &state, &server.client()).await,
        )
        .unwrap();
        let first_stream = &first["response"]["directives"][0]["audioItem"]["stream"];
        assert_eq!(first_stream["token"], "track-s2");
        assert_eq!(first_stream["expectedPreviousToken"], "track-s1");

        let started_request = parse(device_payload(json!({
            "type": "AudioPlayer.PlaybackStarted",
            "token": "track-s2",
            "offsetInMilliseconds": 0
        })));
        let started = serde_json::to_value(handle_playback_started(&started_request, &state));
        assert_eq!(
            started.unwrap(),
            json!({ "version": "1.0", "response": {} })
        );
        assert_eq!(peek_queue(&state, "device-1").index, 1);

        let second_request = parse(device_payload(json!({
            "type": "AudioPlayer.PlaybackNearlyFinished",
            "token": "track-s2",
            "offsetInMilliseconds": 1000
        })));
        let second = serde_json::to_value(
            handle_playback_nearly_finished(&second_request, &state, &server.client()).await,
        )
        .unwrap();
        let second_stream = &second["response"]["directives"][0]["audioItem"]["stream"];
        assert_eq!(second_stream["token"], "track-s3");
        assert_eq!(second_stream["expectedPreviousToken"], "track-s2");
        assert_ne!(second_stream["token"], first_stream["token"]);
    }

    #[tokio::test]
    async fn playback_stopped_for_next_track_resumes_that_tracks_offset() {
        let server = MockServer::start(Vec::new()).await;
        let state = test_state();
        set_queue(
            &state,
            "device-1",
            PlaybackQueue::new(vec!["s1".into(), "s2".into()]),
        );
        let stopped = parse(device_payload(json!({
            "type": "AudioPlayer.PlaybackStopped",
            "token": "track-s2",
            "offsetInMilliseconds": 7300
        })));
        handle_playback_stopped(&stopped, &state);

        let resume = parse(intent_payload_named("AMAZON.ResumeIntent", json!({})));
        let value =
            serde_json::to_value(handle_resume(&resume, &state, &server.client()).await).unwrap();
        let stream = &value["response"]["directives"][0]["audioItem"]["stream"];

        assert_eq!(stream["token"], "track-s2");
        assert_eq!(stream["offsetInMilliseconds"], 7300);
        assert_eq!(peek_queue(&state, "device-1").current_track(), Some("s2"));
    }

    #[test]
    fn handle_playback_stopped_saves_offset() {
        let state = test_state();
        set_queue(
            &state,
            "device-1",
            PlaybackQueue::new(vec!["s1".into(), "s2".into()]),
        );
        let request = parse(device_payload(json!({
            "type": "AudioPlayer.PlaybackStopped",
            "token": "track-s1",
            "offsetInMilliseconds": 4200
        })));

        let value = serde_json::to_value(handle_playback_stopped(&request, &state)).unwrap();

        assert_eq!(value, json!({ "version": "1.0", "response": {} }));
        let queue = peek_queue(&state, "device-1");
        assert_eq!(queue.token.as_deref(), Some("track-s1"));
        assert_eq!(queue.offset_ms, 4_200);
    }

    #[tokio::test]
    async fn handle_play_music_prefers_songs_over_albums() {
        let server = MockServer::start(vec![(
            "search3",
            json!({
                "subsonic-response": {
                    "status": "ok",
                    "searchResult3": {
                        "song": [{ "id": "s7", "title": "Seven", "artist": "Artist" }],
                        "album": [{ "id": "a1", "name": "Album" }],
                        "artist": [{ "id": "r1", "name": "Artist" }]
                    }
                }
            }),
        )])
        .await;
        let state = test_state();
        let request = parse(intent_payload());

        let value = serde_json::to_value(
            handle_request(&request, &server.client(), &state, &test_config()).await,
        )
        .unwrap();

        assert_eq!(
            value["response"]["outputSpeech"]["text"],
            "Reproduciendo Seven de Artist."
        );
        assert_eq!(
            value["response"]["directives"][0]["audioItem"]["stream"]["token"],
            "track-s7"
        );
        let queue = peek_queue(&state, "amzn1.ask.device.789");
        assert_eq!(queue.tracks, vec!["s7".to_owned()]);
    }

    #[tokio::test]
    async fn handle_play_music_falls_back_to_first_album() {
        let server = MockServer::start(vec![
            (
                "search3",
                json!({
                    "subsonic-response": {
                        "status": "ok",
                        "searchResult3": {
                            "album": [{ "id": "a1", "name": "Album" }]
                        }
                    }
                }),
            ),
            (
                "getAlbum",
                json!({
                    "subsonic-response": {
                        "status": "ok",
                        "album": {
                            "song": [{ "id": "a1-s1", "title": "First", "artist": "Band" }]
                        }
                    }
                }),
            ),
        ])
        .await;
        let state = test_state();
        let request = parse(intent_payload());

        let value = serde_json::to_value(
            handle_request(&request, &server.client(), &state, &test_config()).await,
        )
        .unwrap();

        assert_eq!(
            value["response"]["outputSpeech"]["text"],
            "Reproduciendo First de Band."
        );
        let queue = peek_queue(&state, "amzn1.ask.device.789");
        assert_eq!(queue.tracks, vec!["a1-s1".to_owned()]);
    }

    #[tokio::test]
    async fn handle_play_playlist_plays_matching_playlist() {
        let server = MockServer::start(vec![
            (
                "getPlaylists",
                json!({
                    "subsonic-response": {
                        "status": "ok",
                        "playlists": {
                            "playlist": [{ "id": "p1", "name": "Favoritas" }]
                        }
                    }
                }),
            ),
            (
                "getPlaylist",
                json!({
                    "subsonic-response": {
                        "status": "ok",
                        "playlist": {
                            "song": [{ "id": "p1-s1", "title": "One", "artist": "Singer" }]
                        }
                    }
                }),
            ),
        ])
        .await;
        let state = test_state();
        let request = parse(intent_payload_named(
            "PlayPlaylistIntent",
            json!({ "Playlist": { "name": "Playlist", "value": "favor" } }),
        ));

        let value = serde_json::to_value(
            handle_request(&request, &server.client(), &state, &test_config()).await,
        )
        .unwrap();

        assert_eq!(
            value["response"]["outputSpeech"]["text"],
            "Reproduciendo la lista Favoritas."
        );
        assert_eq!(
            value["response"]["directives"][0]["audioItem"]["stream"]["token"],
            "track-p1-s1"
        );
    }

    #[tokio::test]
    async fn handle_request_ignores_unexpected_skill_id() {
        let server = MockServer::start(Vec::new()).await;
        let request = parse(intent_payload());
        let config = Config {
            alexa_skill_id: Some("amzn1.ask.skill.other".into()),
            ..test_config()
        };

        let value = serde_json::to_value(
            handle_request(&request, &server.client(), &test_state(), &config).await,
        )
        .unwrap();

        assert_eq!(value, json!({ "version": "1.0", "response": {} }));
    }

    #[test]
    fn sanitize_search_query_unit() {
        assert_eq!(
            sanitize_search_query("musica de la oreja de van gogh"),
            "la oreja de van gogh"
        );
        assert_eq!(
            sanitize_search_query("pon musica de cancion X"),
            "cancion X"
        );
        assert_eq!(
            sanitize_search_query("reproduce música de la canción"),
            "la canción"
        );
        assert_eq!(sanitize_search_query("no prefix here"), "no prefix here");
    }
}
