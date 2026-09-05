//! One function per CLI verb.

use std::io::IsTerminal;
use std::io::Write;

use crate::config;
use crate::context::Context;
use crate::docsrepo::{self, PushResult, Redo};
use crate::errors::{QuarryError, Result};
use crate::gitcmd;
use crate::identity;
use crate::importer;
use crate::index;
use crate::output::{
    FileRow, FilesOut, InitOut, Meta, Payload, RemoveOut, Response, SyncOut, WriteOut,
};
use crate::query::{self, Direction};

pub(crate) fn init(
    ctx: &Context,
    url: Option<String>,
    docs_dir: Option<String>,
    force: bool,
) -> Result<Response> {
    let existing = ctx.config.clone();
    let mut notes = Vec::new();
    let url = match url {
        Some(url) => Some(url),
        None => match (&existing, std::io::stdin().is_terminal()) {
            (Some(_), _) => None,
            (None, true) => Some(prompt("Docs repo URL: ")?),
            (None, false) => None,
        },
    };
    if let (Some(new), Some(old)) = (url.as_ref(), existing.as_ref())
        && new != &old.url
        && !force
    {
        return Err(QuarryError::refusal(format!(
            "already linked to {}; use --force to relink",
            old.url
        )));
    }
    let git = ctx.repo_git();
    let default_branch = gitcmd::default_branch(&git);
    if ctx.identity.is_some() && default_branch == "main" {
        let has_head = git
            .run_unchecked(&["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"])
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !has_head {
            notes.push("origin/HEAD unset; assuming the default branch is `main`".to_string());
        }
    }
    let config = config::resolve(url, docs_dir, existing.as_ref(), default_branch.clone())?;
    let relinked = existing.as_ref().is_some_and(|old| old.url != config.url);
    let clone_name = identity::clone_name(&config.url)?;
    if relinked {
        let old_name = existing
            .as_ref()
            .map(|c| identity::clone_name(&c.url))
            .transpose()?;
        if let Some(old_name) = old_name {
            let old_path = ctx.quarry_dir().join(&old_name);
            if old_path.exists() {
                std::fs::remove_dir_all(&old_path)?;
                notes.push(format!("removed the previous clone {old_name}"));
            }
        }
    }
    config::write(&ctx.repo_root, &config)?;
    config::write_gitignore(&ctx.repo_root, &clone_name)?;
    let linked = Context {
        config: Some(config.clone()),
        ..ctx.clone()
    };
    if !linked.repo_root.join(&config.docs_dir).exists() {
        notes.push(format!(
            "no {} yet; run Capstone map before quarry add",
            config.docs_dir
        ));
    }
    let cloned = docsrepo::ensure_clone(&linked)?;
    reindex(&linked)?;
    Ok(Response::bare(Payload::Init(InitOut {
        repo: ctx.identity.as_ref().map(|i| i.name.clone()),
        url: config.url,
        docs_dir: config.docs_dir,
        default_branch,
        clone: linked.clone_path()?.display().to_string(),
        cloned,
        notes,
    })))
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    let value = line.trim().to_string();
    if value.is_empty() {
        return Err(QuarryError::refusal("no docs repo URL given"));
    }
    Ok(value)
}

pub(crate) fn add(ctx: &Context) -> Result<Response> {
    import(ctx, true, false)
}

pub(crate) fn update(ctx: &Context, force: bool) -> Result<Response> {
    import(ctx, false, force)
}

fn import(ctx: &Context, adding: bool, force: bool) -> Result<Response> {
    let config = ctx.config()?.clone();
    let identity = ctx.identity()?.clone();
    ctx.require_clone()?;
    let git = ctx.repo_git();
    let head = git.rev_parse("HEAD")?;

    let remote_ref = format!("origin/{}", config.default_branch);
    git.run(&["fetch", "--quiet", "origin", &config.default_branch])?;
    if !git.is_ancestor(&head, &remote_ref)? {
        return Err(QuarryError::refusal(format!(
            "commit {} not on {remote_ref}; merge first",
            short(&head)
        )));
    }

    let mut notes = Vec::new();
    let discarded = docsrepo::refresh(ctx)?;
    if discarded.commits > 0 || discarded.dirty {
        notes.push(format!(
            "discarded {} local commits / changes in the docs clone",
            discarded.commits
        ));
    }
    stamp_sync(ctx)?;

    let folder_existed = ctx.clone_path()?.join(&identity.name).exists();
    if !adding && !folder_existed {
        return Err(QuarryError::refusal(format!(
            "{} is not in the docs repo; run quarry add",
            identity.name
        )));
    }
    if adding && folder_existed {
        notes.push(format!("{} is already in the docs repo", identity.name));
    }
    if adding && !folder_existed {
        let index_page = ctx.repo_root.join(&config.docs_dir).join("00-index.md");
        if !index_page.exists() {
            return Err(QuarryError::refusal(format!(
                "no 00-index.md in {}; run Capstone map first",
                config.docs_dir
            )));
        }
    }

    let outcome = plan(ctx, &identity, &head, force)?;
    let from = outcome.stamp.clone();
    if let Some(reason) = outcome.skip {
        return Ok(Response::bare(Payload::Write(WriteOut {
            repo: identity.name,
            from,
            to: short(&head),
            files: 0,
            result: reason,
            notes,
        })));
    }

    let built = importer::build(ctx, &head)?;
    let files = built.files;
    docsrepo::write_folder(ctx, &identity.name, built.dir.path())?;
    std::mem::forget(built.dir);
    docsrepo::regenerate_root_index(ctx)?;

    let message = format!(
        "{} {} @{}",
        if adding && !folder_existed {
            "add"
        } else {
            "update"
        },
        identity.name,
        short(&head)
    );
    let mut redo_result: Option<String> = None;
    let push = docsrepo::commit_and_push(ctx, &message, || {
        let outcome = plan(ctx, &identity, &head, force)?;
        if let Some(reason) = outcome.skip {
            redo_result = Some(reason.clone());
            return Ok(Redo::Skip(reason));
        }
        let built = importer::build(ctx, &head)?;
        docsrepo::write_folder(ctx, &identity.name, built.dir.path())?;
        std::mem::forget(built.dir);
        docsrepo::regenerate_root_index(ctx)?;
        Ok(Redo::Rebuilt)
    })?;

    let result = match push {
        PushResult::Pushed => "imported".to_string(),
        PushResult::Skipped(reason) => reason,
    };
    reindex(ctx)?;
    Ok(Response::bare(Payload::Write(WriteOut {
        repo: identity.name,
        from,
        to: short(&head),
        files,
        result,
        notes,
    })))
}

struct Plan {
    skip: Option<String>,
    stamp: Option<String>,
}

fn plan(ctx: &Context, identity: &identity::RepoIdentity, head: &str, force: bool) -> Result<Plan> {
    let stamp = docsrepo::read_stamp(ctx, &identity.name)?;
    let Some(stamp) = stamp else {
        return Ok(Plan {
            skip: None,
            stamp: None,
        });
    };
    if let Some(origin) = stamp.origin.as_deref()
        && origin != identity.origin
    {
        return Err(QuarryError::refusal(format!(
            "name {} already used by {origin}",
            identity.name
        )));
    }
    let short_stamp = short(&stamp.commit);
    if stamp.commit == head {
        return Ok(Plan {
            skip: Some("current".to_string()),
            stamp: Some(short_stamp),
        });
    }
    let git = ctx.repo_git();
    if !git.rev_exists(&stamp.commit)? {
        let _ = git.run_unchecked(&["fetch", "--quiet", "origin", &stamp.commit]);
    }
    if !git.rev_exists(&stamp.commit)? {
        if force {
            return Ok(Plan {
                skip: None,
                stamp: Some(short_stamp),
            });
        }
        return Err(QuarryError::refusal(format!(
            "stamp {short_stamp} not reachable from {}; history rewritten? use --force to import anyway",
            short(head)
        )));
    }
    if git.is_ancestor(head, &stamp.commit)? {
        return Ok(Plan {
            skip: Some("skipped".to_string()),
            stamp: Some(short_stamp),
        });
    }
    if git.is_ancestor(&stamp.commit, head)? {
        return Ok(Plan {
            skip: None,
            stamp: Some(short_stamp),
        });
    }
    if force {
        return Ok(Plan {
            skip: None,
            stamp: Some(short_stamp),
        });
    }
    Err(QuarryError::refusal(format!(
        "stamp {short_stamp} diverged from {}; use --force to import anyway",
        short(head)
    )))
}

pub(crate) fn remove(ctx: &Context) -> Result<Response> {
    let identity = ctx.identity()?.clone();
    ctx.require_clone()?;
    docsrepo::refresh(ctx)?;
    stamp_sync(ctx)?;
    let dangling = {
        let index = index::open_current(ctx)?;
        index
            .edges_touching(&identity.name)?
            .into_iter()
            .flat_map(|edge| {
                [edge.from_repo, edge.to_repo]
                    .into_iter()
                    .filter(|r| *r != identity.name)
                    .collect::<Vec<_>>()
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    };
    let removed = docsrepo::remove_folder(ctx, &identity.name)?;
    if !removed {
        return Ok(Response::bare(Payload::Remove(RemoveOut {
            repo: identity.name,
            removed: false,
            dangling,
        })));
    }
    docsrepo::regenerate_root_index(ctx)?;
    docsrepo::commit_and_push(ctx, &format!("remove {}", identity.name), || {
        if !ctx.clone_path()?.join(&identity.name).exists() {
            return Ok(Redo::Skip("removed".to_string()));
        }
        docsrepo::remove_folder(ctx, &identity.name)?;
        docsrepo::regenerate_root_index(ctx)?;
        Ok(Redo::Rebuilt)
    })?;
    reindex(ctx)?;
    Ok(Response::bare(Payload::Remove(RemoveOut {
        repo: identity.name,
        removed: true,
        dangling,
    })))
}

pub(crate) fn sync(ctx: &Context) -> Result<Response> {
    ctx.require_clone()?;
    let before = index::head_of(ctx)?;
    let discarded = docsrepo::refresh(ctx)?;
    let after = index::head_of(ctx)?;
    stamp_sync(ctx)?;
    let opened = index::open_current(ctx)?;
    let mut notes = Vec::new();
    if discarded.commits > 0 || discarded.dirty {
        notes.push(format!(
            "discarded {} local commits / changes in the docs clone",
            discarded.commits
        ));
    }
    let repos = opened.repos()?.len() as u32;
    let report = opened.report.clone();
    Ok(Response::bare(Payload::Sync(SyncOut {
        pulled: before != after,
        index: if report.rebuilt { "rebuilt" } else { "current" }.to_string(),
        repos,
        pages: report.pages,
        edges: report.edges,
        notes,
    })))
}

pub(crate) fn docs_index(ctx: &Context, force: bool) -> Result<Response> {
    let opened = index::open_with(ctx, force)?;
    let mut report = opened.report.clone();
    if !report.rebuilt {
        report.repos = opened.repos()?.len() as u32;
    }
    Ok(Response {
        meta: meta_of(&opened),
        payload: Payload::Index(report),
    })
}

pub(crate) fn docs_list(ctx: &Context, repo: Option<String>) -> Result<Response> {
    let opened = index::open_current(ctx)?;
    let meta = meta_of(&opened);
    match repo {
        None => Ok(Response {
            meta,
            payload: Payload::Repos(query::repos(&opened)?),
        }),
        Some(repo) => {
            let files = query::files(&opened, &repo)?;
            let commit = opened.repo(&repo)?.and_then(|r| r.commit);
            Ok(Response {
                meta,
                payload: Payload::Files(FilesOut {
                    repo,
                    commit,
                    files: files
                        .into_iter()
                        .map(|f| FileRow {
                            path: f.path,
                            generated_date: f.generated_date,
                        })
                        .collect(),
                }),
            })
        }
    }
}

pub(crate) fn docs_show(ctx: &Context, repo: &str) -> Result<Response> {
    let opened = index::open_current(ctx)?;
    Ok(Response {
        meta: meta_of(&opened),
        payload: Payload::Show(Box::new(query::show(&opened, repo)?)),
    })
}

pub(crate) fn docs_section(ctx: &Context, repo: &str, heading: &str) -> Result<Response> {
    let opened = index::open_current(ctx)?;
    Ok(Response {
        meta: meta_of(&opened),
        payload: Payload::Section(Box::new(query::section(&opened, repo, heading)?)),
    })
}

pub(crate) fn docs_search(
    ctx: &Context,
    term: &str,
    repo: Option<&str>,
    limit: u32,
) -> Result<Response> {
    let opened = index::open_current(ctx)?;
    Ok(Response {
        meta: meta_of(&opened),
        payload: Payload::Search(query::search(&opened, term, repo, limit)?),
    })
}

pub(crate) fn docs_deps(
    ctx: &Context,
    repo: &str,
    direction: Direction,
    depth: u32,
) -> Result<Response> {
    let opened = index::open_current(ctx)?;
    Ok(Response {
        meta: meta_of(&opened),
        payload: Payload::Deps(Box::new(query::deps(&opened, repo, direction, depth)?)),
    })
}

pub(crate) fn docs_path(ctx: &Context, from: &str, to: &str) -> Result<Response> {
    let opened = index::open_current(ctx)?;
    Ok(Response {
        meta: meta_of(&opened),
        payload: Payload::Path(Box::new(query::path(&opened, from, to)?)),
    })
}

fn meta_of(index: &index::Index) -> Meta {
    Meta {
        built_at_commit: Some(index.built_at_commit.clone()),
        synced_at: index.synced_at.clone(),
    }
}

fn reindex(ctx: &Context) -> Result<()> {
    let clone = ctx.require_clone()?;
    let head = index::head_of(ctx)?;
    index::rebuild(ctx, &clone, &head)?;
    stamp_sync(ctx)
}

fn stamp_sync(ctx: &Context) -> Result<()> {
    index::set_synced_at(ctx, &jiff::Timestamp::now().to_string())
}

fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}
