use anyhow::Result;
use ratatui::widgets::Block;
use tui_textarea::TextArea;

use crate::{
    cli::Cli, jq::{
        self, JqClient
    }, tokens, scroll_text::ScrollText
};

#[derive(Debug)]
pub struct App {
    /// The original json data from standard in.
    pub original: &'static str,

    /// The current json data as filtered down by the current query.
    pub filtered: String,

    pub scroll_text: ScrollText<'static>,

    /// The current working query
    pub query_editor: TextArea<'static>,

    pub jq_client: JqClient,

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
            scroll_text: ScrollText::from(original.to_string()),
            filtered: original.to_string(),
            query_editor: TextArea::default(),
            jq_client: JqClient::new(),
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
        if let Some(output) = self.jq_client.try_recv_output() {

            match output {
                 jq::JqOutput::Success { json_content } => {
                     log::info!("received a successful response from jq, changing our filtered content now");
                     self.error = None;
                     self.set_display_content(json_content);
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
    pub fn submit_query(&mut self) {
        log::info!("submitting query to jq");
        let query_content = self.query_content().to_string();
        self.jq_client.submit_query(self.original, query_content)
    }

    pub fn set_display_content(&mut self, content: String) {
        // todo: do we need this?
        self.filtered = content.clone();

        let tokens = tokens::tokenize(content.as_str());
        self.scroll_text = ScrollText::from_tokens(tokens.as_slice());
    }

    /// Called when the user scrolls the text area
    pub fn scroll_up(&mut self) {
        log::info!("scroll up");
        self.scroll_text.scroll_up();
    }
    /// Called when the user scrolls the text area
    pub fn scroll_down(&mut self) {
        log::info!("scroll down");
        self.scroll_text.scroll_down();
    }
}
