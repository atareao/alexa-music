//! In-memory playback state, keyed by Alexa device ID.
//!
//! Operations are short and synchronous, so a plain [`std::sync::RwLock`] is
//! enough. Handlers should hold the guard only for the duration of a helper
//! call and never across an `await` point.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use rand::seq::SliceRandom;
use rand::thread_rng;

/// Playback position and ordering for a single device.
#[derive(Debug, Clone)]
pub struct PlaybackQueue {
    /// Subsonic song IDs, in the order they will be played.
    pub tracks: Vec<String>,
    /// Zero-based index of the track currently playing.
    pub index: usize,
    /// `audioPlayer` token of the current stream, when known.
    pub token: Option<String>,
    /// Playback position of the current stream, in milliseconds.
    pub offset_ms: i64,
    /// Whether the remaining portion of the queue has been shuffled.
    pub shuffled: bool,
    /// Original (unshuffled) order of all tracks.
    pub original_order: Vec<String>,
}

/// Shared playback state for all devices handled by the bridge.
pub type AppState = Arc<RwLock<HashMap<String, PlaybackQueue>>>;

impl PlaybackQueue {
    /// Creates a queue that starts at the first track, unshuffled and without
    /// a saved stream position.
    pub fn new(tracks: Vec<String>) -> Self {
        Self {
            original_order: tracks.clone(),
            tracks,
            index: 0,
            token: None,
            offset_ms: 0,
            shuffled: false,
        }
    }

    /// Returns the song ID at the current index, if the queue has one.
    pub fn current_track(&self) -> Option<&str> {
        self.tracks.get(self.index).map(String::as_str)
    }

    /// Returns the queue position of a `track-<song-id>` token, if it belongs
    /// to this queue.
    pub fn position_of_token(&self, token: &str) -> Option<usize> {
        let song_id = token.strip_prefix("track-")?;
        self.tracks.iter().position(|track| track == song_id)
    }

    /// Moves to the next track and returns its song ID.
    ///
    /// Returns `None` (leaving the index unchanged) at the end of the queue.
    pub fn advance(&mut self) -> Option<String> {
        let next = self.index.checked_add(1)?;
        let track = self.tracks.get(next)?.clone();
        self.index = next;
        Some(track)
    }

    /// Moves to the previous track and returns its song ID.
    ///
    /// Returns `None` (leaving the index unchanged) at the start of the queue.
    pub fn retreat(&mut self) -> Option<String> {
        let previous = self.index.checked_sub(1)?;
        let track = self.tracks.get(previous)?.clone();
        self.index = previous;
        Some(track)
    }

    /// Peeks at the next song ID without changing the current index.
    pub fn next_track(&self) -> Option<String> {
        self.tracks.get(self.index.checked_add(1)?).cloned()
    }

    /// Records the `audioPlayer` token and offset used to resume playback.
    pub fn save_offset(&mut self, token: String, offset_ms: i64) {
        self.token = Some(token);
        self.offset_ms = offset_ms;
    }

    /// Randomizes the not-yet-played tail of the queue, keeping already
    /// played tracks in place.
    pub fn shuffle_remaining(&mut self) {
        let start = self.index.saturating_add(1);
        if start < self.tracks.len() {
            self.tracks[start..].shuffle(&mut thread_rng());
        }
        self.shuffled = true;
    }

    /// Restores the not-yet-played tail from [`Self::original_order`],
    /// keeping already played tracks in place.
    pub fn restore_order(&mut self) {
        let index = self.index.min(self.tracks.len());
        if let Some(original_tail) = self.original_order.get(index..) {
            self.tracks.truncate(index);
            self.tracks.extend_from_slice(original_tail);
        }
        self.shuffled = false;
    }
}

/// Returns a clone of the device's queue, or a fresh empty queue if the
/// device has no entry yet. A missing entry is not inserted.
pub fn peek_queue(state: &AppState, device_id: &str) -> PlaybackQueue {
    let guard = state.read().unwrap_or_else(|e| e.into_inner());
    guard
        .get(device_id)
        .cloned()
        .unwrap_or_else(|| PlaybackQueue::new(Vec::new()))
}

/// Stores (or replaces) the queue for a device.
pub fn set_queue(state: &AppState, device_id: &str, queue: PlaybackQueue) {
    let mut guard = state.write().unwrap_or_else(|e| e.into_inner());
    guard.insert(device_id.to_owned(), queue);
}

/// Mutates an existing queue under the write lock and returns its updated
/// clone. Returns `None` when the device has no queue yet.
pub fn update_queue<F>(state: &AppState, device_id: &str, f: F) -> Option<PlaybackQueue>
where
    F: FnOnce(&mut PlaybackQueue),
{
    let mut guard = state.write().unwrap_or_else(|e| e.into_inner());
    let queue = guard.get_mut(device_id)?;
    f(queue);
    Some(queue.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tracks() -> Vec<String> {
        (1..=8).map(|n| format!("song-{n}")).collect()
    }

    #[test]
    fn new_initializes_correctly() {
        let tracks = sample_tracks();
        let queue = PlaybackQueue::new(tracks.clone());

        assert_eq!(queue.tracks, tracks);
        assert_eq!(queue.original_order, tracks);
        assert_eq!(queue.index, 0);
        assert_eq!(queue.token, None);
        assert_eq!(queue.offset_ms, 0);
        assert!(!queue.shuffled);
        assert_eq!(queue.current_track(), Some("song-1"));
    }

    #[test]
    fn advance_and_retreat() {
        let mut queue = PlaybackQueue::new(vec!["a".into(), "b".into(), "c".into()]);

        assert_eq!(queue.advance(), Some("b".into()));
        assert_eq!(queue.index, 1);
        assert_eq!(queue.advance(), Some("c".into()));
        assert_eq!(queue.index, 2);
        assert_eq!(queue.advance(), None);
        assert_eq!(queue.index, 2);

        assert_eq!(queue.retreat(), Some("b".into()));
        assert_eq!(queue.index, 1);
        assert_eq!(queue.retreat(), Some("a".into()));
        assert_eq!(queue.index, 0);
        assert_eq!(queue.retreat(), None);
        assert_eq!(queue.index, 0);

        let empty = PlaybackQueue::new(Vec::new());
        assert_eq!(empty.current_track(), None);
    }

    #[test]
    fn next_track_does_not_change_index() {
        let mut queue = PlaybackQueue::new(vec!["a".into(), "b".into()]);

        assert_eq!(queue.next_track(), Some("b".into()));
        assert_eq!(queue.index, 0);
        assert_eq!(queue.current_track(), Some("a"));

        assert_eq!(queue.advance(), Some("b".into()));
        assert_eq!(queue.next_track(), None);
        assert_eq!(queue.index, 1);
    }

    #[test]
    fn shuffle_and_restore() {
        let original = sample_tracks();
        let mut queue = PlaybackQueue::new(original.clone());
        assert_eq!(queue.advance(), Some("song-2".into()));
        assert_eq!(queue.advance(), Some("song-3".into()));

        queue.shuffle_remaining();

        assert!(queue.shuffled);
        assert_eq!(&queue.tracks[..2], &original[..2]);
        let mut shuffled_tail = queue.tracks[2..].to_vec();
        shuffled_tail.sort();
        let mut original_tail = original[2..].to_vec();
        original_tail.sort();
        assert_eq!(shuffled_tail, original_tail);

        queue.restore_order();

        assert!(!queue.shuffled);
        assert_eq!(queue.tracks, original);
        assert_eq!(queue.current_track(), Some("song-3"));
    }

    #[test]
    fn save_offset_stores_token_and_offset() {
        let mut queue = PlaybackQueue::new(sample_tracks());

        queue.save_offset("token-123".into(), 42_000);

        assert_eq!(queue.token.as_deref(), Some("token-123"));
        assert_eq!(queue.offset_ms, 42_000);
    }

    #[test]
    fn position_of_token_maps_tokens_to_queue_positions() {
        let queue = PlaybackQueue::new(vec!["s1".into(), "s2".into(), "s3".into()]);

        assert_eq!(queue.position_of_token("track-s1"), Some(0));
        assert_eq!(queue.position_of_token("track-s3"), Some(2));
        assert_eq!(queue.position_of_token("track-missing"), None);
        assert_eq!(queue.position_of_token("s2"), None);
    }

    #[test]
    fn state_helpers_work() {
        let state: AppState = Arc::new(RwLock::new(HashMap::new()));

        let missing = peek_queue(&state, "device-1");
        assert!(missing.tracks.is_empty());
        assert_eq!(missing.index, 0);
        // A read-only lookup must not create the entry.
        assert!(state.read().unwrap().is_empty());

        set_queue(&state, "device-1", PlaybackQueue::new(sample_tracks()));
        let stored = peek_queue(&state, "device-1");
        assert_eq!(stored.current_track(), Some("song-1"));

        let updated = update_queue(&state, "device-1", |queue| {
            assert_eq!(queue.advance(), Some("song-2".into()));
            queue.save_offset("token-1".into(), 1_500);
        })
        .unwrap();
        assert_eq!(updated.index, 1);
        assert_eq!(updated.offset_ms, 1_500);

        {
            let stored = state.read().unwrap();
            let queue = stored.get("device-1").unwrap();
            assert_eq!(queue.index, 1);
            assert_eq!(queue.token.as_deref(), Some("token-1"));
        }

        assert!(update_queue(&state, "unknown-device", |_| {}).is_none());
    }
}
