use super::{PlaybackStatus, PlayerState, TrackMetadata};
use std::collections::HashMap;
use std::time::Instant;
use zbus::zvariant::{OwnedValue, Value};
use zbus::Connection;

pub struct MprisClient {
    last_position: f64,
    last_update: Instant,
    last_metadata: TrackMetadata,
}

impl Default for MprisClient {
    fn default() -> Self {
        Self {
            last_position: 0.0,
            last_update: Instant::now(),
            last_metadata: TrackMetadata::default(),
        }
    }
}

impl MprisClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn fetch_current_state(&mut self) -> Option<PlayerState> {
        let conn = Connection::session().await.ok()?;

        let players = list_players(&conn).await?;
        if players.is_empty() {
            return None;
        }

        let player_bus_name = players[0].clone();

        let now = Instant::now();

        // Use GetAll to fetch all Player properties at once — most reliable approach
        let props_reply = conn
            .call_method(
                Some(player_bus_name.as_str()),
                "/org/mpris/MediaPlayer2",
                Some("org.freedesktop.DBus.Properties"),
                "GetAll",
                &("org.mpris.MediaPlayer2.Player"),
            )
            .await;

        let props: HashMap<String, OwnedValue> = match props_reply {
            Ok(msg) => msg.body().deserialize().unwrap_or_default(),
            Err(_) => HashMap::new(),
        };

        // PlaybackStatus
        let status = props
            .get("PlaybackStatus")
            .and_then(|v| extract_str(&**v))
            .map(|s| match s.as_str() {
                "Playing" => PlaybackStatus::Playing,
                "Paused" => PlaybackStatus::Paused,
                _ => PlaybackStatus::Stopped,
            })
            .unwrap_or(PlaybackStatus::Stopped);

        // Metadata dict
        if let Some(meta_owned) = props.get("Metadata") {
            let meta_val: &Value = &**meta_owned;

            if let Some(title) = dict_str(meta_val, "xesam:title") {
                if !title.is_empty() {
                    if title != self.last_metadata.title && !self.last_metadata.title.is_empty() {
                        self.last_position = 0.0;
                        self.last_update = now;
                        self.last_metadata = TrackMetadata::default();
                    }
                    self.last_metadata.title = title;
                }
            }

            if let Some(artist) = dict_artist(meta_val, "xesam:artist") {
                if !artist.is_empty() {
                    self.last_metadata.artist = artist;
                }
            }

            if let Some(album) = dict_str(meta_val, "xesam:album") {
                if !album.is_empty() {
                    self.last_metadata.album = album;
                }
            }

            // mpris:length is in microseconds
            if let Some(length_us) = dict_u64(meta_val, "mpris:length") {
                if length_us > 0 {
                    self.last_metadata.duration_sec = length_us / 1_000_000;
                }
            }
        }

        // Position
        let dbus_pos_us: Option<u64> = props.get("Position").and_then(|v| extract_u64(&**v));

        let mut position_sec = self.last_position;

        if let Some(pos_us) = dbus_pos_us {
            let dbus_pos_sec = (pos_us as f64) / 1_000_000.0;
            if status == PlaybackStatus::Playing {
                let diff = (dbus_pos_sec - self.last_position).abs();
                if diff > 0.8 || dbus_pos_sec < self.last_position {
                    // Seek/jump detected
                    position_sec = dbus_pos_sec;
                    self.last_position = dbus_pos_sec;
                    self.last_update = now;
                } else if diff >= 0.01 {
                    position_sec = dbus_pos_sec;
                    self.last_position = dbus_pos_sec;
                    self.last_update = now;
                } else {
                    let elapsed = now.duration_since(self.last_update).as_secs_f64();
                    position_sec = self.last_position + elapsed;
                }
            } else {
                position_sec = dbus_pos_sec;
                self.last_position = dbus_pos_sec;
                self.last_update = now;
            }
        } else if status == PlaybackStatus::Playing {
            let elapsed = now.duration_since(self.last_update).as_secs_f64();
            position_sec = self.last_position + elapsed;
        }

        if self.last_metadata.duration_sec > 0 {
            position_sec = position_sec.min(self.last_metadata.duration_sec as f64);
        }

        Some(PlayerState {
            metadata: self.last_metadata.clone(),
            position_sec,
        })
    }
}

// --- Player discovery helpers ---

async fn list_players(conn: &Connection) -> Option<Vec<String>> {
    let reply = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "ListNames",
            &(),
        )
        .await
        .ok()?;
    let (names,): (Vec<String>,) = reply.body().deserialize().ok()?;

    let mut players: Vec<String> = names
        .into_iter()
        .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
        // playerctld is a proxy forwarding the active player — not a real
        // source, listing it would just duplicate the actual player
        .filter(|n| !clean_player_name(n).eq_ignore_ascii_case("playerctld"))
        .collect();
    players.sort();
    Some(players)
}

fn clean_player_name(bus_name: &str) -> String {
    bus_name
        .trim_start_matches("org.mpris.MediaPlayer2.")
        .split('.')
        .next()
        .unwrap_or("Media Player")
        .to_string()
}

// --- OwnedValue helpers ---

fn extract_str(val: &Value) -> Option<String> {
    match val {
        Value::Str(s) => Some(s.as_str().to_string()),
        Value::Value(inner) => extract_str(inner),
        _ => None,
    }
}

fn extract_u64(val: &Value) -> Option<u64> {
    match val {
        Value::U64(u) => Some(*u),
        Value::I64(i) => Some((*i).max(0) as u64),
        Value::U32(u) => Some(*u as u64),
        Value::I32(i) => Some((*i).max(0) as u64),
        Value::Value(inner) => extract_u64(inner),
        _ => None,
    }
}

// --- Dict `a{sv}` helpers ---

fn dict_str(val: &Value, key: &str) -> Option<String> {
    let entry = dict_get(val, key)?;
    extract_str(entry)
}

fn dict_u64(val: &Value, key: &str) -> Option<u64> {
    let entry = dict_get(val, key)?;
    extract_u64(entry)
}

fn dict_artist(val: &Value, key: &str) -> Option<String> {
    let entry = dict_get(val, key)?;
    extract_artist(entry)
}

fn extract_artist(val: &Value) -> Option<String> {
    match val {
        Value::Array(arr) => {
            let names: Vec<String> = arr.iter().filter_map(extract_str).collect();
            if names.is_empty() {
                None
            } else {
                Some(names.join(", "))
            }
        }
        Value::Value(inner) => extract_artist(inner),
        _ => extract_str(val),
    }
}

fn dict_get<'a>(val: &'a Value<'a>, key: &str) -> Option<&'a Value<'a>> {
    match val {
        Value::Dict(dict) => {
            for (k, v) in dict.iter() {
                let k_str = match k {
                    Value::Str(s) => s.as_str(),
                    _ => continue,
                };
                if k_str == key {
                    return Some(v);
                }
            }
            None
        }
        Value::Value(inner) => dict_get(inner, key),
        _ => None,
    }
}
