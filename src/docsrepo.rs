//! The docs repo clone: refresh, stamps, folder swap, root index, push.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::errors::{QuarryError, Result};
use crate::gitcmd::Git;
use crate::index::{RepoScan, scan_repo_dir};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Stamp {
    pub(crate) commit: String,
    #[serde(default)]
    pub(crate) origin: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Discarded {
    pub(crate) commits: u32,
    pub(crate) dirty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PushResult {
    Pushed,
    Skipped(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Redo {
    Rebuilt,
    Skip(String),
}

pub(crate) const STAMP_FILE: &str = ".quarry-stamp";

pub(crate) fn ensure_clone(ctx: &Context) -> Result<bool> {
    let path = ctx.clone_path()?;
    if path.join(".git").exists() {
        refresh(ctx)?;
        return Ok(false);
    }
    fs::create_dir_all(ctx.quarry_dir())?;
    let url = ctx.config()?.url.clone();
    let dest = path.to_string_lossy().to_string();
    Git::new(&ctx.repo_root, ctx.verbose).run(&["clone", "--depth", "1", &url, &dest])?;
    // Cloning an empty repository leaves no fetch refspec behind.
    let clone = Git::new(&path, ctx.verbose);
    if clone
        .stdout(&["config", "--get", "remote.origin.fetch"])
        .unwrap_or_default()
        .is_empty()
    {
        clone.run(&[
            "config",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/origin/*",
        ])?;
    }
    Ok(true)
}

pub(crate) fn docs_branch(git: &Git) -> String {
    match git.stdout(&["symbolic-ref", "--short", "HEAD"]) {
        Ok(name) if !name.is_empty() => name,
        _ => "main".to_string(),
    }
}

pub(crate) fn refresh(ctx: &Context) -> Result<Discarded> {
    let git = ctx.clone_git()?;
    let branch = docs_branch(&git);
    let fetched = git.run_unchecked(&["fetch", "--depth", "1", "origin", &branch])?;
    if !fetched.status.success() {
        let stderr = String::from_utf8_lossy(&fetched.stderr).to_string();
        if !stderr.contains("couldn't find remote ref") {
            return Err(QuarryError::external(format!(
                "git fetch failed: {}",
                stderr.trim()
            )));
        }
        return Ok(Discarded::default());
    }
    let dirty = !git.stdout(&["status", "--porcelain"])?.is_empty();
    let mut commits = local_commits(&git);
    if commits > 0 && ctx.clone_path()?.join(".git/shallow").exists() {
        // A shallow history makes "behind the remote" look like local work.
        let _ = git.run_unchecked(&["fetch", "--deepen", "50", "origin", &branch]);
        commits = local_commits(&git);
    }
    git.run(&["reset", "--hard", "--quiet", "FETCH_HEAD"])?;
    git.run(&["clean", "-qfd"])?;
    Ok(Discarded { commits, dirty })
}

pub(crate) fn read_stamp(ctx: &Context, repo: &str) -> Result<Option<Stamp>> {
    let path = ctx.clone_path()?.join(repo).join(STAMP_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)?;
    match serde_json::from_str::<Stamp>(&text) {
        Ok(stamp) => Ok(Some(stamp)),
        Err(_) => Ok(None),
    }
}

pub(crate) fn stamp_json(commit: &str, origin: &str) -> String {
    format!("{{\"commit\":\"{commit}\",\"origin\":\"{origin}\"}}\n")
}

pub(crate) fn write_folder(ctx: &Context, repo: &str, built: &Path) -> Result<()> {
    let dest = ctx.clone_path()?.join(repo);
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    fs::rename(built, &dest)?;
    Ok(())
}

pub(crate) fn remove_folder(ctx: &Context, repo: &str) -> Result<bool> {
    let dest = ctx.clone_path()?.join(repo);
    if !dest.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(&dest)?;
    Ok(true)
}

pub(crate) fn regenerate_root_index(ctx: &Context) -> Result<()> {
    let clone = ctx.clone_path()?;
    let title = clone
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "quarry".to_string());
    let mut rows: Vec<(String, RepoScan)> = Vec::new();
    for entry in read_repo_dirs(&clone)? {
        let name = entry
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let scan = scan_repo_dir(&entry)?;
        rows.push((name, scan));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut text = format!("# {title}\n\n");
    text.push_str("| Repo | Imported | Newest doc | Pages | Produces | Consumes |\n");
    text.push_str("|---|---|---|---|---|---|\n");
    for (name, scan) in rows {
        let imported = scan
            .commit
            .as_deref()
            .map(|c| c.chars().take(7).collect::<String>())
            .unwrap_or_else(|| "-".to_string());
        let newest = scan
            .newest_generated_date
            .unwrap_or_else(|| "-".to_string());
        text.push_str(&format!(
            "| [{name}]({name}/00-index.md) | {imported} | {newest} | {} | {} | {} |\n",
            scan.pages, scan.produces, scan.consumes
        ));
    }
    fs::write(clone.join("00-index.md"), text)?;
    Ok(())
}

pub(crate) fn local_commits(git: &Git) -> u32 {
    git.stdout(&["rev-list", "FETCH_HEAD..HEAD", "--count"])
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0)
}

pub(crate) fn read_repo_dirs(clone: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(clone)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join("00-index.md").exists() {
            dirs.push(path);
        }
    }
    dirs.sort();
    Ok(dirs)
}

pub(crate) fn commit_and_push(
    ctx: &Context,
    message: &str,
    mut redo: impl FnMut() -> Result<Redo>,
) -> Result<PushResult> {
    let git = ctx.clone_git()?;
    let branch = docs_branch(&git);
    let refspec = format!("HEAD:{branch}");
    for _attempt in 1..=3 {
        git.run(&["add", "-A"])?;
        let staged = git.run_unchecked(&["diff", "--cached", "--quiet"])?;
        if staged.status.success() {
            return Ok(PushResult::Skipped("current".to_string()));
        }
        git.run(&["commit", "-q", "-m", message])?;
        let out = git.run_unchecked(&["push", "origin", &refspec])?;
        if out.status.success() {
            return Ok(PushResult::Pushed);
        }
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        if !(stderr.contains("rejected") || stderr.contains("fetch first")) {
            refresh(ctx)?;
            return Err(QuarryError::external(format!(
                "push failed: {}",
                stderr.trim()
            )));
        }
        refresh(ctx)?;
        if let Redo::Skip(reason) = redo()? {
            return Ok(PushResult::Skipped(reason));
        }
    }
    Err(QuarryError::external("docs repo busy, retry"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn s1_stamp_bytes_are_stable() {
        assert_eq!(
            stamp_json("abc", "github.com/acme/ingest-api"),
            "{\"commit\":\"abc\",\"origin\":\"github.com/acme/ingest-api\"}\n"
        );
        let parsed: Stamp = serde_json::from_str(&stamp_json("abc", "o")).unwrap();
        assert_eq!(parsed.commit, "abc");
        assert_eq!(parsed.origin.as_deref(), Some("o"));
    }
}
