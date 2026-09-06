use crate::lyrics::{LyricsData, LyricsService};
use crate::player::mpris::MprisClient;
use crate::player::PlayerState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct App {
    pub player: PlayerState,
    pub lyrics: LyricsData,
    pub lyrics_service: LyricsService,
    pub mpris: MprisClient,
    pub manual_scroll_offset: Option<isize>,
    pub should_quit: bool,
    pub last_fetched_song: (String, String),
}

impl App {
    pub fn new() -> Self {
        Self {
            player: PlayerState::default(),
            lyrics: LyricsData::empty(),
            lyrics_service: LyricsService::new(),
            mpris: MprisClient::new(),
            manual_scroll_offset: None,
            should_quit: false,
            last_fetched_song: (String::new(), String::new()),
        }
    }

    pub async fn on_tick(&mut self) {
        if let Some(state) = self.mpris.fetch_current_state().await {
            let pos_diff = (state.position_sec - self.player.position_sec).abs();
            if pos_diff > 1.5 {
                self.manual_scroll_offset = None;
            }
            self.player = state;
        }

        let current_title = self.player.metadata.title.clone();
        let current_artist = self.player.metadata.artist.clone();

        let song_key = (current_artist.clone(), current_title.clone());
        if !current_title.is_empty() && song_key != self.last_fetched_song {
            self.last_fetched_song = song_key;
            self.fetch_lyrics_for_current_song().await;
        }
    }

    pub async fn fetch_lyrics_for_current_song(&mut self) {
        let artist = self.player.metadata.artist.clone();
        let title = clean_title(&self.player.metadata.title);
        let album_owned = if self.player.metadata.album.is_empty() {
            None
        } else {
            Some(self.player.metadata.album.clone())
        };
        let dur = if self.player.metadata.duration_sec > 0 {
            Some(self.player.metadata.duration_sec)
        } else {
            None
        };

        let fetched = self
            .lyrics_service
            .get_lyrics(&artist, &title, album_owned.as_deref(), dur)
            .await;

        self.lyrics = fetched;
        self.manual_scroll_offset = None;
    }

    pub async fn handle_key_event(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                let current = self.manual_scroll_offset.unwrap_or(0);
                self.manual_scroll_offset = Some(current - 1);
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                let current = self.manual_scroll_offset.unwrap_or(0);
                self.manual_scroll_offset = Some(current + 1);
            }
            (KeyCode::Char('c'), _) => {
                self.manual_scroll_offset = None;
            }
            (KeyCode::Char('r'), _) => {
                if self.player.metadata.title.is_empty() {
                    return;
                }
                let artist = self.player.metadata.artist.clone();
                let title = clean_title(&self.player.metadata.title);
                self.lyrics_service.invalidate(&artist, &title).await;
                self.fetch_lyrics_for_current_song().await;
            }
            _ => {}
        }
    }
}

fn clean_title(title: &str) -> String {
    let t = title.trim();
    let start = match (t.rfind('('), t.rfind('[')) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    if let Some(s) = start {
        let close = if t[s..].starts_with('(') { ')' } else { ']' };
        if let Some(rel) = t[s..].find(close) {
            let inner = t[s + 1..s + rel].to_lowercase();
            let noise = ["official", "video", "lyric", "audio", "hd", "remaster", "live", "acoustic"];
            if noise.iter().any(|w| inner.contains(w)) {
                return t[..s].trim_end().to_string();
            }
        }
    }
    t.to_string()
}
