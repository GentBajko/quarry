//! quarry: one docs repo for many code repos, queryable by agents and people.

mod check;
mod cli;
mod commands;
mod config;
mod context;
mod docsrepo;
mod errors;
mod frontmatter;
mod gitcmd;
mod identity;
mod importer;
mod index;
mod observed;
mod output;
mod query;
mod secrets;

use std::io::Write;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::Cli;
use crate::errors::QuarryError;

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => return usage_exit(e),
    };
    match cli::run(&cli) {
        Ok(response) => {
            let note = if cli.json {
                None
            } else {
                stale_note(&response)
            };
            let text = output::render(&response, cli.json, note);
            if write_out(&text).is_err() {
                return ExitCode::from(2);
            }
            ExitCode::from(response.exit_code())
        }
        Err(e) => fail(&e, cli.json),
    }
}

fn stale_note(response: &output::Response) -> Option<String> {
    output::stale_note(response.meta.synced_at.as_deref(), jiff::Timestamp::now())
        .filter(|_| response.meta.built_at_commit.is_some())
}

fn write_out(text: &str) -> std::io::Result<()> {
    let mut stdout = std::io::stdout();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()
}

fn fail(error: &QuarryError, json: bool) -> ExitCode {
    if json {
        let _ = write_out(&output::error_envelope(error));
    } else {
        eprintln!("{error}");
    }
    ExitCode::from(error.exit_code())
}

fn usage_exit(error: clap::Error) -> ExitCode {
    use clap::error::ErrorKind;
    match error.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            let _ = write_out(&error.to_string());
            ExitCode::SUCCESS
        }
        _ => {
            eprint!("{error}");
            ExitCode::from(1)
        }
    }
}
