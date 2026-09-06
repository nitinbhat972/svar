#[derive(Debug, Clone)]
pub struct LrcLine {
    pub time_sec: f64,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct LyricsData {
    pub synced: bool,
    pub lines: Vec<LrcLine>,
    pub plain_lines: Vec<String>,
}

impl LyricsData {
    pub fn empty() -> Self {
        Self {
            synced: false,
            lines: Vec::new(),
            plain_lines: vec!["No lyrics available".to_string()],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
            && (self.plain_lines.is_empty()
                || (self.plain_lines.len() == 1 && self.plain_lines[0] == "No lyrics available"))
    }
}

pub fn parse_lrc(lrc_content: &str) -> LyricsData {
    let mut lines = Vec::new();
    let mut plain_lines = Vec::new();
    let mut is_synced = false;

    for line in lrc_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            plain_lines.push(String::new());
            continue;
        }

        // Parse timestamp [mm:ss.xx] or [mm:ss:xx] or [mm:ss]
        let mut parsed_any = false;
        let mut text = trimmed;

        while text.starts_with('[') {
            if let Some(close_bracket) = text.find(']') {
                let tag = &text[1..close_bracket];
                if let Some(time) = parse_timestamp(tag) {
                    parsed_any = true;
                    is_synced = true;
                    text = text[close_bracket + 1..].trim();
                    lines.push(LrcLine {
                        time_sec: time,
                        text: text.to_string(),
                    });
                } else {
                    // Could be meta tag like [ar: Artist] or [ti: Title]
                    break;
                }
            } else {
                break;
            }
        }

        if !parsed_any && !trimmed.starts_with('[') {
            plain_lines.push(trimmed.to_string());
        }
    }

    if is_synced {
        lines.sort_by(|a, b| {
            a.time_sec
                .partial_cmp(&b.time_sec)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        LyricsData {
            synced: true,
            lines,
            plain_lines,
        }
    } else {
        LyricsData {
            synced: false,
            lines: Vec::new(),
            plain_lines,
        }
    }
}

fn parse_timestamp(tag: &str) -> Option<f64> {
    let (min_str, sec_part) = tag.split_once(':')?;
    let min: f64 = min_str.parse().ok()?;
    let sec: f64 = if let Some((s_str, ms_str)) = sec_part.split_once('.') {
        let s: f64 = s_str.parse().ok()?;
        let ms: f64 = ms_str.parse::<f64>().ok()? / 10_f64.powi(ms_str.len() as i32);
        s + ms
    } else {
        sec_part.parse().ok()?
    };
    Some(min * 60.0 + sec)
}
