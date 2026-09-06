use super::lrc::{parse_lrc, LyricsData};
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LrclibResponse {
    pub instrumental: Option<bool>,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
}

pub struct LrclibClient {
    client: reqwest::Client,
}

impl LrclibClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .user_agent("svar/0.1.0")
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub async fn fetch_lyrics(
        &self,
        artist: &str,
        title: &str,
        album: Option<&str>,
        duration_sec: Option<u64>,
    ) -> Result<LyricsData> {
        // Try direct get endpoint first
        if let Ok(data) = self.get_exact(artist, title, album, duration_sec).await {
            return Ok(data);
        }

        // Search endpoint fallback
        self.search_fallback(artist, title).await
    }

    async fn get_exact(
        &self,
        artist: &str,
        title: &str,
        album: Option<&str>,
        duration_sec: Option<u64>,
    ) -> Result<LyricsData> {
        let mut url = format!(
            "https://lrclib.net/api/get?track_name={}&artist_name={}",
            urlencoding::encode(title),
            urlencoding::encode(artist)
        );

        if let Some(alb) = album {
            if !alb.is_empty() {
                url.push_str(&format!("&album_name={}", urlencoding::encode(alb)));
            }
        }

        if let Some(dur) = duration_sec {
            if dur > 0 {
                url.push_str(&format!("&duration={}", dur));
            }
        }

        let resp = self.client.get(&url).send().await?;

        if resp.status().is_success() {
            let item: LrclibResponse = resp.json().await?;
            return Self::process_response(&item);
        }

        Err(anyhow!("LRCLIB get request returned {}", resp.status()))
    }

    async fn search_fallback(&self, artist: &str, title: &str) -> Result<LyricsData> {
        let query = format!("{} {}", artist, title);
        let url = format!(
            "https://lrclib.net/api/search?q={}",
            urlencoding::encode(&query)
        );

        let resp = self.client.get(&url).send().await?;

        if resp.status().is_success() {
            let items: Vec<LrclibResponse> = resp.json().await?;
            for item in items {
                if let Ok(data) = Self::process_response(&item) {
                    if !data.is_empty() {
                        return Ok(data);
                    }
                }
            }
        }

        Err(anyhow!("No lyrics found on LRCLIB"))
    }

    fn process_response(item: &LrclibResponse) -> Result<LyricsData> {
        if item.instrumental.unwrap_or(false) {
            return Ok(LyricsData {
                synced: false,
                lines: Vec::new(),
                plain_lines: vec!["♪ Instrumental Track ♪".to_string()],
            });
        }

        if let Some(ref synced) = item.synced_lyrics {
            if !synced.trim().is_empty() {
                let parsed = parse_lrc(synced);
                if !parsed.lines.is_empty() {
                    return Ok(parsed);
                }
            }
        }

        if let Some(ref plain) = item.plain_lyrics {
            if !plain.trim().is_empty() {
                let lines: Vec<String> = plain.lines().map(|l| l.to_string()).collect();
                return Ok(LyricsData {
                    synced: false,
                    lines: Vec::new(),
                    plain_lines: lines,
                });
            }
        }

        Err(anyhow!("Empty lyrics in response"))
    }
}
