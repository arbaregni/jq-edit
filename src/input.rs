use anyhow::Result;
use ratatui::
    crossterm::
        event::{
            self, 
            Event,
            KeyCode,
            KeyEvent, KeyEventKind,
        }
;

use crate::app::App;

const POLL_DURATION: std::time::Duration = std::time::Duration::from_millis(50);

pub fn handle_events(app: &mut App) -> Result<()> {
    if !event::poll(POLL_DURATION)? {
        return Ok(());
    }

    // Process the event. The query editor should be shown every input, except for Esc and Enter
    // because we are hiding those from the text area
    let ev = event::read()?;
    match ev {
        // Quite the app on `Esc`
        Event::Key(KeyEvent { kind, code: KeyCode::Esc, .. }) => {
            if kind == KeyEventKind::Press {
                app.run = false;
            }
        }
        // Submit a new query on "enter"
        Event::Key(KeyEvent { kind, code: KeyCode::Enter, .. }) => {
            if kind == KeyEventKind::Press {
                app.submit_message();
            }
        },
        Event::Key(KeyEvent { code: KeyCode::Tab | KeyCode::BackTab, .. }) => {
            // nothing to do, just need to intercept this from text area edit
        }
        ev => {
            app.query_editor.input(ev);
        }
    };

    Ok(())
}
