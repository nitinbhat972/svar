pub mod lrc;
pub mod lrclib;

pub use lrc::LyricsData;

pub use lrclib::LrclibClient;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct LyricsService {
    lrclib: Arc<LrclibClient>,
    cache: Arc<Mutex<HashMap<String, LyricsData>>>,
}

impl Default for LyricsService {
    fn default() -> Self {
        Self {
            lrclib: Arc::new(LrclibClient::new()),
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl LyricsService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn invalidate(&self, artist: &str, title: &str) {
        let cache_key = cache_key(artist, title);
        self.cache.lock().await.remove(&cache_key);
    }

    pub async fn get_lyrics(
        &self,
        artist: &str,
        title: &str,
        album: Option<&str>,
        duration_sec: Option<u64>,
    ) -> LyricsData {
        let cache_key = cache_key(artist, title);

        {
            let cache = self.cache.lock().await;
            if let Some(data) = cache.get(&cache_key) {
                return data.clone();
            }
        }

        // LRCLIB provides synced lyrics (syncedLyrics) or unsynced plain lyrics (plainLyrics)
        if let Ok(data) = self
            .lrclib
            .fetch_lyrics(artist, title, album, duration_sec)
            .await
        {
            if !data.is_empty() {
                let mut cache = self.cache.lock().await;
                cache.insert(cache_key, data.clone());
                return data;
            }
        }

        LyricsData::empty()
    }
}

fn cache_key(artist: &str, title: &str) -> String {
    format!(
        "{} - {}",
        artist.trim().to_lowercase(),
        title.trim().to_lowercase()
    )
}
