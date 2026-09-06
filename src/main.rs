mod app;
mod lyrics;
mod player;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::event::{Event, EventStream};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal).await;
    ratatui::restore();
    result
}

async fn run_app(terminal: &mut DefaultTerminal) -> Result<()> {
    let mut app = App::new();

    let mut reader = EventStream::new();
    let mut interval = tokio::time::interval(Duration::from_millis(50));

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        tokio::select! {
            _ = interval.tick() => {
                app.on_tick().await;
            }
            maybe_event = reader.next() => {
                if let Some(Ok(Event::Key(key))) = maybe_event {
                    app.handle_key_event(key).await;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn print_help() {
    println!(
        "{name} {version}\n{desc}\n\nUsage: {name}\n\nOptions:\n  -h, --help   Print this help message\n\nKeys:\n  q / Ctrl+C   Quit\n  j / Down     Scroll down\n  k / Up       Scroll up\n  c            Re-center live lyrics\n  r            Re-fetch lyrics",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
        desc = env!("CARGO_PKG_DESCRIPTION"),
    );
}
