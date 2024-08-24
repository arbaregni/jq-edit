use anyhow::Result;
use tui_textarea::TextArea;

use crate::{
    cli::Cli,
    jq::{
        self, JqJob
    }
};

#[derive(Debug)]
pub struct App {
    /// The original json data from standard in.
    pub original: &'static str,

    /// The current json data as filtered down by the current query.
    pub filtered: String,

    /// The current working query
    pub query_editor: TextArea<'static>,

    pub running_query: Option<JqJob>,

    /// True while the app should be running.
    pub is_running: bool,
    
    /// Present if there is some error message to display
    pub error: Option<ErrorPanel>,

    /// Seems that after an interaction with jq, we should clear the screen and force a redraw
    pub clear_screen: bool,
}

#[derive(Debug)]
pub struct ErrorPanel {
    pub title: String,
    pub failure: String,
}

impl App {
    pub fn init(original: &'static str) -> App {
        App {
            original,
            filtered: original.to_string(),
            query_editor: TextArea::default(),
            running_query: None,
            is_running: true,
            error: None,
            clear_screen: false,
        }
    }

    pub fn filtered_content(&self) -> &str {
        self.filtered.as_str()
    }

    pub fn query_content(&self) -> &str {
        self.query_editor.lines()[0].as_str()
    }

    pub fn update(&mut self, _cli: &Cli) -> Result<()> {

        // check if the job is done running
        if let Some(output) = JqJob::extract_output_once_done(&mut self.running_query) {

            match output {
                 jq::JqOutput::Success { json_content } => {
                     log::info!("received a successful response from jq, changing our filtered content now");
                     self.filtered = json_content;
                     self.error = None;
                 }
                 jq::JqOutput::Failure { title, failure } => {
                     // do NOT overwrite previous content on a fail, just show last good state
                     log::info!("received an error from jq");
                     self.error = Some(ErrorPanel {
                         title,
                         failure
                    });
                 }
             }

            self.clear_screen = true;
        }

        Ok(())
    }

    /// Called when the user presses enter. Runs the query again
    pub fn submit_message(&mut self) {
        log::info!("submitting query to jq");
        let job = JqJob::new(self.original, self.query_content().to_string());
        self.running_query = Some(job);
    }
}
