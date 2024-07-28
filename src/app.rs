use anyhow::Result;
use tui_textarea::TextArea;

use crate::{cli::Cli, jq};

#[derive(Debug)]
pub struct App {
    /// The original json data from standard in.
    pub original: String,

    /// The current json data as filtered down by the current query.
    pub filtered: String,

    /// The current working query
    pub query_editor: TextArea<'static>,

    /// True if the query buf has changed since last update
    pub query_changed: bool,

    /// True while the app should be running.
    pub run: bool,
    
    /// Present if there is some error message to display
    pub error: Option<ErrorPanel>,
}

#[derive(Debug)]
pub struct ErrorPanel {
    pub title: String,
    pub failure: String,
}

impl App {
    pub fn init(original: String) -> App {
        App {
            original: original.clone(),
            filtered: original,
            query_editor: TextArea::default(),
            query_changed: true,
            run: true,
            error: None,
        }
    }

    pub fn query_content(&self) -> &str {
        self.query_editor.lines()[0].as_str()
    }
    pub fn update(&mut self, cli: &Cli) -> Result<()> {
        // update the filters

        if self.query_changed {
            let out = jq::apply_filter(&cli, self.original.as_str(), self.query_content())?;

            match out {
                 jq::JqOutput::Success { json_content } => {
                     self.filtered = json_content;
                     self.error = None;
                 }
                 jq::JqOutput::Failure { title, failure } => {
                     // do NOT overwrite previous content on a fail, just show last good state
                     self.error = Some(ErrorPanel {
                         title,
                         failure
                    });
                 }
             }

            self.query_changed = false;
        }

        Ok(())
    }

    /// Called when the user presses enter. Runs the query again
    pub fn submit_message(&mut self) {
        self.query_changed = true;
    }
}
