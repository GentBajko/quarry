//! The clap tree and the dispatch into `commands`.

use clap::{ArgGroup, CommandFactory, Parser, Subcommand};

use crate::commands;
use crate::context::{Context, ContextArgs};
use crate::errors::Result;
use crate::output::{Payload, Response};
use crate::query::Direction;

const SUPPORT: &str =
    "Support quarry: https://patreon.com/quarry | https://buymeacoffee.com/quarry";

/// One docs repo for many code repos, queryable by agents and people.
#[derive(Debug, Parser)]
#[command(name = "quarry", version, about, after_help = SUPPORT)]
pub(crate) struct Cli {
    /// Machine-readable output: one JSON document on stdout.
    #[arg(long, global = true)]
    pub(crate) json: bool,
    /// Echo every git command to stderr.
    #[arg(long, global = true)]
    pub(crate) verbose: bool,
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Link this repo to a docs repo and clone it.
    Init {
        /// The docs repo URL.
        #[arg(long, env = "QUARRY_DOCS_REPO")]
        url: Option<String>,
        /// The docs folder in this repo.
        #[arg(long, env = "QUARRY_DOCS_DIR")]
        docs_dir: Option<String>,
        /// This repo's default branch, when origin/HEAD does not say.
        #[arg(long, env = "QUARRY_DEFAULT_BRANCH")]
        default_branch: Option<String>,
        /// Relink to a different docs repo.
        #[arg(long)]
        force: bool,
    },
    /// Register this repo in the docs repo and import its docs.
    Add {
        /// Refuse the import when a declared site is not in the tree at HEAD.
        #[arg(long)]
        strict: bool,
    },
    /// Copy this repo's docs into the docs repo, commit, push.
    Update {
        /// Import over a diverged or unreachable stamp.
        #[arg(long)]
        force: bool,
        /// Refuse the import when a declared site is not in the tree at HEAD.
        #[arg(long)]
        strict: bool,
    },
    /// Pull the docs repo clone and rebuild the index.
    Sync,
    /// Drop this repo from the docs repo.
    Remove,
    /// Query the docs repo.
    Docs {
        #[command(subcommand)]
        command: Option<DocsCommand>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum DocsCommand {
    /// Repos in the docs repo, or one repo's files.
    List {
        /// Limit the listing to one repo.
        repo: Option<String>,
    },
    /// One repo's stamps, aliases, edges, publications, and overview.
    Show {
        /// The repo name.
        repo: String,
    },
    /// One section by its heading, never a whole file.
    Section {
        /// The repo name.
        repo: String,
        /// The heading to return.
        heading: String,
    },
    /// Full-text search over sections.
    Search {
        /// The term or phrase to look for.
        term: String,
        /// Restrict the search to one repo.
        #[arg(long)]
        repo: Option<String>,
        /// Maximum hits to print.
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    #[command(group(ArgGroup::new("dir").required(true).args(["downstream", "upstream"])))]
    /// Who consumes what this repo produces (declared edges, then possible consumers by name), or the other way round.
    Deps {
        /// The repo name.
        repo: String,
        /// Follow producer to consumer.
        #[arg(long)]
        downstream: bool,
        /// Follow consumer to producer.
        #[arg(long)]
        upstream: bool,
        /// How far to walk; 0 is unlimited.
        #[arg(long, default_value_t = 1)]
        depth: u32,
    },
    /// The shortest chain of edges between two repos.
    Path {
        /// The repo the chain starts at.
        from: String,
        /// The repo the chain ends at.
        to: String,
    },
    /// Rebuild the local index and list edge targets that resolve to nothing.
    Index {
        /// Rebuild regardless of the stamps.
        #[arg(long)]
        force: bool,
    },
}

pub(crate) fn run(cli: &Cli) -> Result<Response> {
    let Some(command) = &cli.command else {
        return Ok(Response::bare(Payload::Help(help_text(None))));
    };
    if let Command::Docs { command: None } = command {
        return Ok(Response::bare(Payload::Help(help_text(Some("docs")))));
    }
    let ctx = Context::build(&ContextArgs {
        verbose: cli.verbose,
    })?;
    match command {
        Command::Init {
            url,
            docs_dir,
            default_branch,
            force,
        } => commands::init(
            &ctx,
            given(url),
            given(docs_dir),
            given(default_branch),
            *force,
        ),
        Command::Add { strict } => commands::add(&ctx, *strict),
        Command::Update { force, strict } => commands::update(&ctx, *force, *strict),
        Command::Sync => commands::sync(&ctx),
        Command::Remove => commands::remove(&ctx),
        Command::Docs { command } => match command {
            None => Ok(Response::bare(Payload::Help(help_text(Some("docs"))))),
            Some(DocsCommand::List { repo }) => commands::docs_list(&ctx, repo.clone()),
            Some(DocsCommand::Show { repo }) => commands::docs_show(&ctx, repo),
            Some(DocsCommand::Section { repo, heading }) => {
                commands::docs_section(&ctx, repo, heading)
            }
            Some(DocsCommand::Search { term, repo, limit }) => {
                commands::docs_search(&ctx, term, repo.as_deref(), *limit)
            }
            Some(DocsCommand::Deps {
                repo,
                downstream,
                depth,
                ..
            }) => commands::docs_deps(
                &ctx,
                repo,
                if *downstream {
                    Direction::Downstream
                } else {
                    Direction::Upstream
                },
                *depth,
            ),
            Some(DocsCommand::Path { from, to }) => commands::docs_path(&ctx, from, to),
            Some(DocsCommand::Index { force }) => commands::docs_index(&ctx, *force),
        },
    }
}

fn given(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub(crate) fn help_text(subcommand: Option<&str>) -> String {
    let mut command = Cli::command();
    match subcommand {
        None => command.render_help().to_string(),
        Some(name) => match command.find_subcommand_mut(name) {
            Some(sub) => sub.render_help().to_string(),
            None => command.render_help().to_string(),
        },
    }
}
