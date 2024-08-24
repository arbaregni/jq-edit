
use crate::cli::Cli;

use anyhow::Result;
use subprocess::{
    ExitStatus, Popen, PopenConfig, Redirection
};

const JQ_EXE_NAME: &str = "jq";

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

pub fn apply_filter(_cli: &Cli, source: &str, query: &str) -> Result<JqOutput> {
    let mut process = Popen::create(
        &[JQ_EXE_NAME, query],
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

    log::info!("stdout = {stdout}, stderr = {stderr}");

    // translate the shell program's output
    let output = match exit_status {
        ExitStatus::Exited(rc) => match rc {
            0 => JqOutput::Success {
                json_content: stdout
            },
            _ => JqOutput::Failure {
                title: format!("jq subprocess exittied with exit code {rc}"),
                failure: stderr,
            }
        },
        ExitStatus::Signaled(x) => JqOutput::Failure {
            title: format!("error"),
            failure: format!("the jq subprocess exited due to a signal {x}")
        },
        ExitStatus::Other(x) => JqOutput::Failure {
            title: format!("error"),
            failure: format!("This should not occur. The jq subprocess exited (other - {x})"),
        },
        ExitStatus::Undetermined => JqOutput::Failure {
            title: format!("error"),
            failure: format!("undetermined exit status of jq subprocess")
        },
    };

    Ok(output)
}
