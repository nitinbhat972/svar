use crate::lyrics::LyricsData;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

fn style(fg: Color) -> Style {
    Style::default().fg(fg)
}

fn style_dim(fg: Color) -> Style {
    Style::default().fg(fg).add_modifier(Modifier::DIM)
}

fn centered_msg(frame: &mut Frame, area: Rect, text: &str, msg_style: Style) {
    let msg = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(msg_style);
    let middle_y = area.height.saturating_sub(1) / 2;
    let msg_area = Rect::new(area.x, area.y + middle_y, area.width, 1);
    frame.render_widget(msg, msg_area);
}

pub fn render_lyrics(
    frame: &mut Frame,
    area: Rect,
    lyrics: &LyricsData,
    position_sec: f64,
    manual_scroll_offset: Option<isize>,
) {
    if lyrics.is_empty() {
        centered_msg(frame, area, "No lyrics found", style_dim(Color::Reset));
        return;
    }

    if lyrics.synced {
        render_synced_lyrics(frame, area, lyrics, position_sec, manual_scroll_offset);
    } else {
        render_plain_lyrics(frame, area, lyrics, manual_scroll_offset);
    }
}

fn render_synced_lyrics(
    frame: &mut Frame,
    area: Rect,
    lyrics: &LyricsData,
    position_sec: f64,
    manual_scroll_offset: Option<isize>,
) {
    let current_idx = lyrics
        .lines
        .iter()
        .rposition(|line| line.time_sec <= position_sec)
        .unwrap_or(0);

    let height = (area.height as usize).saturating_sub(1);
    let auto_scroll = current_idx.saturating_sub(height / 2);

    let final_scroll = if let Some(offset) = manual_scroll_offset {
        let max_scroll = (lyrics.lines.len() as isize - 1).max(0);
        ((auto_scroll as isize) + offset).clamp(0, max_scroll) as usize
    } else {
        auto_scroll
    };

    let lines: Vec<Line> = lyrics
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let style = if i == current_idx {
                style(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                style_dim(Color::Reset)
            };
            Line::from(Span::styled(line.text.as_str(), style))
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .scroll((final_scroll as u16, 0));

    frame.render_widget(paragraph, area);
}

fn render_plain_lyrics(
    frame: &mut Frame,
    area: Rect,
    lyrics: &LyricsData,
    manual_scroll_offset: Option<isize>,
) {
    if lyrics.plain_lines.len() == 1 && lyrics.plain_lines[0] == "♪ Instrumental Track ♪" {
        centered_msg(
            frame,
            area,
            "♪ Instrumental Track ♪",
            style_dim(Color::Reset).add_modifier(Modifier::ITALIC),
        );
        return;
    }

    let max_scroll = (lyrics.plain_lines.len() as isize - 1).max(0);
    let offset = manual_scroll_offset.unwrap_or(0).clamp(0, max_scroll) as u16;

    let lines: Vec<Line> = lyrics
        .plain_lines
        .iter()
        .map(|l| Line::from(Span::styled(l.as_str(), style(Color::Reset))))
        .collect();

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .scroll((offset, 0));

    frame.render_widget(paragraph, area);
}
