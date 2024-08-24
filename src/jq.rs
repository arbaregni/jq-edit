
use crate::cli::Cli;

use std::sync::mpsc::{
    channel, Receiver, Sender, TryRecvError
};
use std::thread;

use anyhow::Result;
use subprocess::{
    ExitStatus, Popen, PopenConfig, Redirection
};

const JQ_EXE_NAME: &str = "jq";

#[derive(Debug)]
pub struct JqJob {
    rx: Receiver<JqOutput>
}

impl JqJob {
    pub fn new(source: &'static str, query: String) -> JqJob {
        let (tx, rx) = channel();
        thread::spawn(move || {
            log::info!("spawning jq worker thread");
            let result = apply_filter(source, query);
            let out = match result {
                Ok(out) => out,
                Err(e) => {
                    log::error!("jq worker exitted with error: {e}");
                    JqOutput::Failure {
                        title: format!("fault"),
                        failure: format!("jq worker exitted with error: {e}")
                    }
                }
            };
            match tx.send(out) {
                Ok(_) => {},
                Err(e) => {
                    log::error!("could not send jq output: {e}");
                }
            }
        });
        JqJob { rx }
    }
    /// Get the output of the command, if it is ready
    pub fn output(&self) -> Option<JqOutput> {
        match self.rx.try_recv() {
            Ok(out) => Some(out),
            Err(TryRecvError::Disconnected) => Some(JqOutput::Failure {
                title: format!("fault"),
                failure: format!("channel to jq worker thread disconnected")
            }),
            Err(TryRecvError::Empty) => None,
        }

    }
    pub fn extract_output_once_done(maybe_job: &mut Option<JqJob>) -> Option<JqOutput> {
        let Some(job) = maybe_job else { return None; };
        let Some(output) = job.output() else { return None; };

        *maybe_job = None;
        Some(output)
    }
}

#[derive(Debug)]
pub enum JqOutput {
    Success {
        json_content: String
    },
    Failure {
        title: String,
        failure: String,
    },
}
fn apply_filter(source: &'static str, query: String) -> Result<JqOutput> {
    let mut process = Popen::create(
        &[JQ_EXE_NAME, query.as_str()],
        PopenConfig {
            stdin: Redirection::Pipe,
            stdout: Redirection::Pipe,
            stderr: Redirection::Pipe,
            ..Default::default()
        }
    )?;

    let (stdout, stderr) = process.communicate(Some(source))?;
    let exit_status = process.wait()?;

    log::info!("jq exitted with {exit_status:?}");

    let stdout = stdout.unwrap_or(format!("<missing stdout>"));
    let stderr = stderr.unwrap_or(format!("<missing stderr>"));

    log::info!("jq stdout = {}", stdout.replace("\n", "\\n"));

    // translate the shell program's output
    let output = match exit_status {
        ExitStatus::Exited(rc) => match rc {
            0 => JqOutput::Success {
                json_content: stdout
            },
            _ => JqOutput::Failure {
                title: format!("jq subprocess exited with exit code {rc}"),
                failure: stderr,
            }
        },
        ExitStatus::Signaled(x) => JqOutput::Failure {
            title: format!("fault"),
            failure: format!("the jq subprocess exited due to a signal {x}")
        },
        ExitStatus::Other(x) => JqOutput::Failure {
            title: format!("fault"),
            failure: format!("This should not occur. The jq subprocess exited (other - {x})"),
        },
        ExitStatus::Undetermined => JqOutput::Failure {
            title: format!("fault"),
            failure: format!("undetermined exit status of jq subprocess")
        },
    };

    Ok(output)
}


