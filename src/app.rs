use anyhow::Result;

use crate::{cli::Cli, jq};

#[derive(Debug)]
pub struct App {
    /// The original json data from standard in.
    pub original: String,

    /// The current json data as filtered down by the current query.
    pub filtered: String,

    /// The current working query
    pub query_buf: String,

    /// True if the query buf has changed since last update
    pub query_changed: bool,

    /// True while the app should be running.
    pub run: bool,
    
    /// Present if there is some error message to display
    pub error: Option<String>,
}


impl App {
    pub fn init(original: String) -> App {
        App {
            original: original.clone(),
            filtered: original,
            query_buf: String::with_capacity(10),
            query_changed: true,
            run: true,
            error: None,
        }
    }

    pub fn update(&mut self, cli: &Cli) -> Result<()> {
        // update the filters

        if self.query_changed {
            let out = jq::apply_filter(&cli, self.original.as_str(), self.query_buf.as_str())?;

            match out {
                 jq::JqOutput::Success { json_content } => {
                     self.filtered = json_content;
                     self.error = None;
                 }
                 jq::JqOutput::Failure { failure } => {
                     self.error = Some(failure);
                 }
             }

            self.query_changed = false;
        }

        Ok(())
    }

}
