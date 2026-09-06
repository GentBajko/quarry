//! One function per CLI verb.

use std::collections::BTreeSet;
use std::io::IsTerminal;
use std::io::Write;

use crate::check;
use crate::config;
use crate::context::Context;
use crate::docsrepo::{self, PushResult, Redo};
use crate::errors::{QuarryError, Result};
use crate::frontmatter;
use crate::gitcmd;
use crate::identity;
use crate::importer;
use crate::index;
use crate::output::{
    FileRow, FilesOut, InitOut, Meta, Payload, RemoveOut, Response, SyncOut, WriteOut, count_breaks,
};
use crate::query::{self, Direction};

pub(crate) fn init(
    ctx: &Context,
    url: Option<String>,
    docs_dir: Option<String>,
    name: Option<String>,
    branch: Option<String>,
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
    if let Some(wanted) = name.as_deref()
        && ctx.identity.as_ref().is_some_and(|i| i.name == wanted)
    {
        return Err(QuarryError::refusal(format!(
            "target {wanted} is this repo's own name; the umbrella folder uses it"
        )));
    }
    let git = ctx.repo_git();
    let derived = gitcmd::default_branch(&git);
    let config = config::resolve(url, docs_dir, name, branch, existing.as_ref(), derived)?;
    let default_branch = config.default_branch.clone();
    if ctx.identity.is_some() && !gitcmd::remote_branch_known(&git, &default_branch) {
        notes.push(format!(
            "no origin/{default_branch} here; set the right one with quarry init --default-branch <name>"
        ));
    }
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
    // With targets configured the umbrella is optional, so the root folder is
    // not worth a note; each target's own folder is.
    if config.targets.is_empty() {
        if !linked.repo_root.join(&config.docs_dir).exists() {
            notes.push(format!(
                "no {} yet; run Capstone map before quarry add",
                config.docs_dir
            ));
        }
    } else {
        for target in &config.targets {
            if !linked.repo_root.join(&target.docs_dir).exists() {
                notes.push(format!(
                    "no {} yet for target {}; run Capstone map before quarry add",
                    target.docs_dir, target.name
                ));
            }
        }
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
        targets: config.targets,
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

pub(crate) fn add(ctx: &Context, strict: bool) -> Result<Response> {
    import(ctx, true, false, strict)
}

pub(crate) fn update(ctx: &Context, force: bool, strict: bool) -> Result<Response> {
    import(ctx, false, force, strict)
}

/// One folder in the docs repo: a monorepo target, or the whole repo when no
/// targets are configured, or the root index-of-indexes.
struct Unit {
    name: String,
    docs_dir: String,
    umbrella: bool,
}

impl Unit {
    fn target(&self) -> config::Target {
        config::Target {
            name: self.name.clone(),
            docs_dir: self.docs_dir.clone(),
        }
    }
}

// A hand-edited config never reaches a write: the names and dirs are validated
// on every load, not only when `init` writes them.
fn units(
    config: &config::Config,
    identity: &identity::RepoIdentity,
    root_index_present: bool,
) -> Result<Vec<Unit>> {
    if config.targets.is_empty() {
        return Ok(vec![Unit {
            name: identity.name.clone(),
            docs_dir: config.docs_dir.clone(),
            umbrella: false,
        }]);
    }
    config::validate_targets(&config.docs_dir, &config.targets)?;
    let mut units = Vec::new();
    for target in &config.targets {
        if target.name == identity.name {
            return Err(QuarryError::refusal(format!(
                "target {} is this repo's own name; the umbrella folder uses it",
                target.name
            )));
        }
        units.push(Unit {
            name: target.name.clone(),
            docs_dir: target.docs_dir.clone(),
            umbrella: false,
        });
    }
    if root_index_present {
        units.push(Unit {
            name: identity.name.clone(),
            docs_dir: config.docs_dir.clone(),
            umbrella: true,
        });
    }
    Ok(units)
}

fn umbrella_of<'a>(unit: &Unit, config: &'a config::Config) -> &'a [config::Target] {
    if unit.umbrella { &config.targets } else { &[] }
}

// The umbrella page's links are rewritten from the configured target list, so
// registering a target changes its content even when the source commit has not
// moved. The folders in the clone stamped with this repo's origin are what the
// last import saw; a difference means the links are behind the config.
fn umbrella_is_stale(
    ctx: &Context,
    config: &config::Config,
    identity: &identity::RepoIdentity,
) -> Result<bool> {
    if config.targets.is_empty() {
        return Ok(false);
    }
    let clone = ctx.clone_path()?;
    let mut imported: BTreeSet<String> = BTreeSet::new();
    for dir in docsrepo::read_repo_dirs(&clone)? {
        let Some(name) = dir.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name == identity.name {
            continue;
        }
        let origin = docsrepo::read_stamp(ctx, name)?.and_then(|stamp| stamp.origin);
        if origin.is_some_and(|origin| origin == identity.origin) {
            imported.insert(name.to_string());
        }
    }
    let configured: BTreeSet<String> = config.targets.iter().map(|t| t.name.clone()).collect();
    Ok(imported != configured)
}

fn import(ctx: &Context, adding: bool, force: bool, strict: bool) -> Result<Response> {
    let config = ctx.config()?.clone();
    let identity = ctx.identity()?.clone();
    let root_index_present = ctx
        .repo_root
        .join(&config.docs_dir)
        .join("00-index.md")
        .exists();
    let units = units(&config, &identity, root_index_present)?;
    // The payload shape follows the configuration, not the unit count: a repo
    // with one target and no umbrella still answers with an array, so a CI
    // consumer reading `.result[0]` keeps working when a workspace is added.
    let per_target = !config.targets.is_empty();
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

    // The discarded note describes the shared clone, so every unit carries it.
    let mut shared_notes = Vec::new();
    let discarded = docsrepo::refresh(ctx)?;
    if discarded.commits > 0 || discarded.dirty {
        shared_notes.push(format!(
            "discarded {} local commits / changes in the docs clone",
            discarded.commits
        ));
    }
    stamp_sync(ctx)?;

    let clone = ctx.clone_path()?;
    let mut existed = Vec::with_capacity(units.len());
    let mut outs: Vec<WriteOut> = Vec::with_capacity(units.len());
    for unit in &units {
        let folder_existed = clone.join(&unit.name).exists();
        if !adding && !folder_existed {
            return Err(QuarryError::refusal(format!(
                "{} is not in the docs repo; run quarry add",
                unit.name
            )));
        }
        let mut notes = shared_notes.clone();
        if adding && folder_existed {
            notes.push(format!("{} is already in the docs repo", unit.name));
        }
        if adding && !folder_existed {
            let index_page = ctx.repo_root.join(&unit.docs_dir).join("00-index.md");
            if !index_page.exists() {
                return Err(QuarryError::refusal(format!(
                    "no 00-index.md in {}; run Capstone map first",
                    unit.docs_dir
                )));
            }
        }
        existed.push(folder_existed);
        outs.push(WriteOut {
            repo: unit.name.clone(),
            from: None,
            to: short(&head),
            files: 0,
            result: String::new(),
            notes,
        });
    }

    let mut pending: Vec<usize> = Vec::new();
    let umbrella_stale = umbrella_is_stale(ctx, &config, &identity)?;
    for (i, unit) in units.iter().enumerate() {
        let outcome = plan(ctx, &unit.name, &unit.docs_dir, &identity, &head, force)?;
        outs[i].from = outcome.stamp;
        match outcome.skip {
            // A stamp on the source commit says nothing about the target list
            // the umbrella's links were rewritten from.
            Some(reason) if unit.umbrella && umbrella_stale && reason == "current" => {
                pending.push(i)
            }
            Some(reason) => outs[i].result = reason,
            None => pending.push(i),
        }
    }
    if pending.is_empty() {
        return Ok(Response::bare(payload_of(per_target, outs)));
    }

    // Every pending unit is built before any folder is written, so a refusal in
    // the second target leaves the clone untouched.
    let mut built = Vec::with_capacity(pending.len());
    for &i in &pending {
        built.push(importer::build(
            ctx,
            &head,
            &units[i].target(),
            umbrella_of(&units[i], &config),
        )?);
    }
    // The secret gate goes first: of the two things --strict refuses, a leaked
    // credential is the one to read about before anything else.
    secret_gate(&built, strict)?;
    if strict {
        let unverified: Vec<importer::UnverifiedSite> = built
            .iter()
            .flat_map(|b| b.unverified.iter().cloned())
            .collect();
        if !unverified.is_empty() {
            return Err(QuarryError::refusal(strict_message(&unverified, &head)));
        }
    }
    for (n, &i) in pending.iter().enumerate() {
        outs[i].files = built[n].files;
        outs[i].notes.extend(
            built[n]
                .unverified
                .iter()
                .map(|site| importer::unverified_note(site, &head)),
        );
        outs[i]
            .notes
            .extend(built[n].secrets.iter().map(secret_note));
        docsrepo::write_folder(ctx, &units[i].name, built[n].dir.path())?;
    }
    for one in built {
        std::mem::forget(one.dir);
    }
    docsrepo::regenerate_root_index(ctx)?;

    let verb = if adding && pending.iter().any(|&i| !existed[i]) {
        "add"
    } else {
        "update"
    };
    let names: Vec<&str> = pending.iter().map(|&i| units[i].name.as_str()).collect();
    let message = format!("{verb} {} @{}", names.join(", "), short(&head));
    let mut redo_skips: Vec<Option<String>> = vec![None; pending.len()];
    let push = docsrepo::commit_and_push(ctx, &message, || {
        let mut rebuilt: Vec<(usize, importer::Built)> = Vec::new();
        let mut first_reason: Option<String> = None;
        // The clone was reset onto whatever the other pusher left, so the
        // umbrella's target set is read again.
        let umbrella_stale = umbrella_is_stale(ctx, &config, &identity)?;
        for (n, &i) in pending.iter().enumerate() {
            let outcome = plan(
                ctx,
                &units[i].name,
                &units[i].docs_dir,
                &identity,
                &head,
                force,
            )?;
            let forced =
                units[i].umbrella && umbrella_stale && outcome.skip.as_deref() == Some("current");
            if let Some(reason) = outcome.skip.filter(|_| !forced) {
                if first_reason.is_none() {
                    first_reason = Some(reason.clone());
                }
                redo_skips[n] = Some(reason);
                continue;
            }
            redo_skips[n] = None;
            rebuilt.push((
                i,
                importer::build(
                    ctx,
                    &head,
                    &units[i].target(),
                    umbrella_of(&units[i], &config),
                )?,
            ));
        }
        // The redo rebuilds the same commit, so the verdict is the one already
        // reached; running it again keeps the guarantee that nothing strict
        // refuses is ever written, whatever a later change makes the redo build.
        secret_gate(rebuilt.iter().map(|(_, one)| one), strict)?;
        if rebuilt.is_empty() {
            return Ok(Redo::Skip(
                first_reason.unwrap_or_else(|| "current".to_string()),
            ));
        }
        for (i, one) in &rebuilt {
            docsrepo::write_folder(ctx, &units[*i].name, one.dir.path())?;
        }
        for (_, one) in rebuilt {
            std::mem::forget(one.dir);
        }
        docsrepo::regenerate_root_index(ctx)?;
        Ok(Redo::Rebuilt)
    })?;

    let result = match push {
        PushResult::Pushed => "imported".to_string(),
        PushResult::Skipped(reason) => reason,
    };
    for (n, &i) in pending.iter().enumerate() {
        outs[i].result = redo_skips[n].clone().unwrap_or_else(|| result.clone());
    }
    reindex(ctx)?;
    // One open for the whole run: the index was just rebuilt and every unit
    // reads the same database.
    let opened = index::open_current(ctx)?;
    for &i in &pending {
        let notes = contract_notes(ctx, &opened, &units[i].name)?;
        outs[i].notes.extend(notes);
    }
    Ok(Response::bare(payload_of(per_target, outs)))
}

// No targets keeps today's single-object payload; targets make it an array,
// however many folders this run touched.
fn payload_of(per_target: bool, mut outs: Vec<WriteOut>) -> Payload {
    if !per_target && outs.len() == 1 {
        Payload::Write(outs.remove(0))
    } else {
        Payload::WriteMany(outs)
    }
}

pub(crate) fn check(ctx: &Context) -> Result<Response> {
    let config = ctx.config()?.clone();
    let identity = ctx.identity()?.clone();
    let clone = ctx.require_clone()?;
    let root_index_present = ctx
        .repo_root
        .join(&config.docs_dir)
        .join("00-index.md")
        .exists();
    // The umbrella folder is an index-of-indexes; it never carries a chapter to
    // check, so it is dropped without a note.
    let checked: Vec<Unit> = units(&config, &identity, root_index_present)?
        .into_iter()
        .filter(|unit| !unit.umbrella)
        .collect();
    // A configured target is the producer name consumers write and the folder
    // the docs repo holds; the repo's own name is not a folder at all when the
    // root index is absent. So the target is named whenever targets exist, not
    // only when there are two or more of them.
    let per_target = !config.targets.is_empty();
    let opened = index::open_current(ctx)?;
    let mut merged = check::CheckOut {
        repo: identity.name.clone(),
        contracts: Vec::new(),
        breaks: Vec::new(),
        warnings: Vec::new(),
        notes: Vec::new(),
    };
    for unit in &checked {
        let page = ctx
            .repo_root
            .join(&unit.docs_dir)
            .join(frontmatter::INTERFACES_PAGE);
        let text = match std::fs::read_to_string(&page) {
            Ok(text) => Some(text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(QuarryError::from(e)),
        };
        if text.is_none() && per_target {
            merged.notes.push(format!(
                "{}: no {}/{}; nothing to check",
                unit.name,
                unit.docs_dir,
                frontmatter::INTERFACES_PAGE
            ));
            continue;
        }
        let mut out = check::run(&opened, &unit.name, text.as_deref(), &clone)?;
        // An unregistered repo already carries check::run's own note; a second
        // one about the missing chapter would add nothing. The edge filter is
        // the same three terms check::run applies when it groups consumers.
        if text.is_none() && opened.has_repo(&unit.name)? {
            let has_consumers = opened.edges(&unit.name, true)?.iter().any(|edge| {
                !edge.missing && edge.declared_by != "observed" && edge.to_repo != unit.name
            });
            let note = if has_consumers {
                format!(
                    "no {}/{} in the working tree; run Capstone map first",
                    unit.docs_dir,
                    frontmatter::INTERFACES_PAGE
                )
            } else {
                // check::run's no-consumers note says the same thing from the
                // other side; B3 words this case as one note.
                let redundant = format!(
                    "no consumers of {} in the quarry; nothing to compare",
                    unit.name
                );
                out.notes.retain(|n| n != &redundant);
                format!(
                    "no {} in {}; nothing to check",
                    frontmatter::INTERFACES_PAGE,
                    unit.docs_dir
                )
            };
            out.notes.insert(0, note);
        }
        if per_target {
            for contract in &mut out.contracts {
                contract.target = Some(unit.name.clone());
            }
            for one in &mut out.breaks {
                one.target = Some(unit.name.clone());
            }
        }
        merged.contracts.extend(out.contracts);
        merged.breaks.extend(out.breaks);
        merged.warnings.extend(out.warnings);
        merged.notes.extend(out.notes);
    }
    Ok(Response {
        meta: meta_of(&opened),
        payload: Payload::Check(merged),
    })
}

// The imported folder is in the clone by now, so this compares the pages that
// were just written. Advisory: breaks become notes and the exit code is
// unchanged.
fn contract_notes(ctx: &Context, index: &index::Index, repo: &str) -> Result<Vec<String>> {
    let clone = ctx.require_clone()?;
    let page = clone.join(repo).join(frontmatter::INTERFACES_PAGE);
    let text = match std::fs::read_to_string(&page) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(QuarryError::from(e)),
    };
    let out = check::run(index, repo, Some(&text), &clone)?;
    let mut notes: Vec<String> = out
        .breaks
        .iter()
        .map(|b| {
            format!(
                "contract check: {} reads {} from {} {}; {}",
                b.consumer, b.field, b.kind, b.name, b.reason
            )
        })
        .collect();
    if !notes.is_empty() {
        notes.push(format!(
            "contract check: {}",
            count_breaks(out.breaks.len())
        ));
    }
    Ok(notes)
}

struct Plan {
    skip: Option<String>,
    stamp: Option<String>,
}

fn plan(
    ctx: &Context,
    folder: &str,
    docs_dir: &str,
    identity: &identity::RepoIdentity,
    head: &str,
    force: bool,
) -> Result<Plan> {
    let stamp = docsrepo::read_stamp(ctx, folder)?;
    let Some(stamp) = stamp else {
        return Ok(Plan {
            skip: None,
            stamp: None,
        });
    };
    // A stamp written before 0.2.0 came from the root docs dir by definition,
    // and only the current config knows which dir that is.
    if let Some(origin) = stamp.origin.as_deref() {
        let root = ctx.config()?.docs_dir.clone();
        let stamped_dir = stamp.docs_dir.as_deref().unwrap_or(&root);
        if origin != identity.origin || config::tidy_dir(stamped_dir) != config::tidy_dir(docs_dir)
        {
            return Err(QuarryError::refusal(format!(
                "name {folder} already used by {origin} at {stamped_dir}"
            )));
        }
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
    let config = ctx.config()?.clone();
    let identity = ctx.identity()?.clone();
    let per_target = !config.targets.is_empty();
    // The umbrella folder may sit in the docs repo even when the working tree
    // has lost its root index, so removal considers it whenever the clone holds
    // it. A repo whose root docs dir never had an index has no such folder and
    // gets no block for one.
    let units = units(&config, &identity, true)?;
    let clone = ctx.require_clone()?;
    docsrepo::refresh(ctx)?;
    stamp_sync(ctx)?;
    let units: Vec<Unit> = units
        .into_iter()
        .filter(|unit| !unit.umbrella || clone.join(&unit.name).exists())
        .collect();
    let going: BTreeSet<String> = units.iter().map(|unit| unit.name.clone()).collect();
    let dangling: Vec<Vec<String>> = {
        let index = index::open_current(ctx)?;
        let mut per_unit = Vec::with_capacity(units.len());
        for unit in &units {
            // A sibling folder this same run drops is not left dangling.
            per_unit.push(
                index
                    .edges_touching(&unit.name)?
                    .into_iter()
                    .filter(|edge| edge.declared_by != "observed")
                    .flat_map(|edge| [edge.from_repo, edge.to_repo])
                    .filter(|repo| !going.contains(repo))
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>(),
            );
        }
        per_unit
    };
    let mut outs = Vec::with_capacity(units.len());
    let mut removed_names: Vec<&str> = Vec::new();
    for (i, unit) in units.iter().enumerate() {
        let removed = docsrepo::remove_folder(ctx, &unit.name)?;
        if removed {
            removed_names.push(unit.name.as_str());
        }
        outs.push(RemoveOut {
            repo: unit.name.clone(),
            removed,
            dangling: dangling[i].clone(),
        });
    }
    if removed_names.is_empty() {
        return Ok(Response::bare(remove_payload(per_target, outs)));
    }
    docsrepo::regenerate_root_index(ctx)?;
    let message = format!("remove {}", removed_names.join(", "));
    docsrepo::commit_and_push(ctx, &message, || {
        let clone = ctx.clone_path()?;
        let mut again = false;
        for unit in &units {
            if clone.join(&unit.name).exists() {
                docsrepo::remove_folder(ctx, &unit.name)?;
                again = true;
            }
        }
        if !again {
            return Ok(Redo::Skip("removed".to_string()));
        }
        docsrepo::regenerate_root_index(ctx)?;
        Ok(Redo::Rebuilt)
    })?;
    reindex(ctx)?;
    Ok(Response::bare(remove_payload(per_target, outs)))
}

fn remove_payload(per_target: bool, mut outs: Vec<RemoveOut>) -> Payload {
    if !per_target && outs.len() == 1 {
        Payload::Remove(outs.remove(0))
    } else {
        Payload::RemoveMany(outs)
    }
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
        report.observed = opened.observed.clone();
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

fn secret_note(hit: &crate::secrets::SecretHit) -> String {
    format!("{} matches the {} shape", hit.file, hit.pattern)
}

// Hits are collected across every pending unit before the first folder is
// written, so a secret in the second target leaves the first one unwritten too.
fn secret_gate<'a>(
    built: impl IntoIterator<Item = &'a importer::Built>,
    strict: bool,
) -> Result<()> {
    if !strict {
        return Ok(());
    }
    let mut lines: Vec<String> = built
        .into_iter()
        .flat_map(|one| one.secrets.iter())
        .map(secret_note)
        .collect();
    if lines.is_empty() {
        return Ok(());
    }
    lines.push("remove them from the docs before importing".to_string());
    Err(QuarryError::refusal(lines.join("\n")))
}

fn strict_message(sites: &[importer::UnverifiedSite], head: &str) -> String {
    let mut lines: Vec<String> = sites
        .iter()
        .map(|site| importer::unverified_note(site, head))
        .collect();
    lines.push("fix 09-interfaces.md or run without --strict".to_string());
    lines.join("\n")
}
