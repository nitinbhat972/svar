pub mod lyrics;

use crate::app::App;
use lyrics::render_lyrics;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
    Frame,
};

fn style(fg: Color) -> Style {
    Style::default().fg(fg)
}

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_lyrics(
        frame,
        chunks[1],
        &app.lyrics,
        app.player.position_sec,
        app.manual_scroll_offset,
    );

    let has_lyrics = !app.lyrics.is_empty()
        && !(app.lyrics.plain_lines.len() == 1
            && app.lyrics.plain_lines[0] == "♪ Instrumental Track ♪");

    if !app.lyrics.synced && has_lyrics && !app.player.metadata.title.is_empty() {
        let badge = Paragraph::new("Not Synced")
            .alignment(Alignment::Right)
            .style(style(Color::Yellow).add_modifier(Modifier::ITALIC | Modifier::BOLD));
        frame.render_widget(badge, chunks[0]);
    }
}
