use anyhow::Result;
use ratatui::
    crossterm::
        event::{self, Event, KeyCode}
;

use crate::app::App;

const POLL_DURATION: std::time::Duration = std::time::Duration::from_millis(50);

pub fn handle_events(app: &mut App) -> Result<()> {
    if !event::poll(POLL_DURATION)? {
        return Ok(());
    }

    match event::read()? {
        Event::Key(key) if key.kind == event::KeyEventKind::Press => match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                app.run = false;
            }
            _ => { /* nothing to do */ }
        }
        _ => { /* nothing to do */ }
    }


    Ok(())
}
