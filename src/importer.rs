//! Copies the docs folder at one commit, with pinned permalinks.

use std::collections::HashSet;
use std::fs;
use std::sync::LazyLock;

use regex::{Captures, Regex};
use tempfile::TempDir;

use crate::context::Context;
use crate::docsrepo::{STAMP_FILE, stamp_json};
use crate::errors::{QuarryError, Result};

pub(crate) const HOST_TEMPLATES: &[(&str, &str)] = &[
    (
        "github.com",
        "https://{host}/{owner}/{repo}/blob/{sha}/{path}#L{line}",
    ),
    (
        "gitlab.com",
        "https://{host}/{owner}/{repo}/-/blob/{sha}/{path}#L{line}",
    ),
];

const DEFAULT_TEMPLATE: &str = "https://{host}/{owner}/{repo}/blob/{sha}/{path}#L{line}";

const POINTER_PATTERN: &str = r"`(?P<path>[^`\s:]+):(?P<a>\d+)(?:-(?P<b>\d+))?`";

static POINTER: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(POINTER_PATTERN).ok());

#[derive(Debug)]
pub(crate) struct Built {
    pub(crate) dir: TempDir,
    pub(crate) files: usize,
}

pub(crate) fn build(ctx: &Context, sha: &str) -> Result<Built> {
    let config = ctx.config()?;
    let identity = ctx.identity()?;
    let git = ctx.repo_git();
    let dir = tempfile::Builder::new()
        .prefix(".build-")
        .tempdir_in(ctx.quarry_dir())?;
    let prefix = format!("{sha}:{}", config.docs_dir);
    let listed = git.run_unchecked(&["ls-tree", "-r", "--name-only", &prefix])?;
    if !listed.status.success() {
        return Err(QuarryError::refusal(format!(
            "no {} at {sha}; run Capstone map first",
            config.docs_dir
        )));
    }
    let names: Vec<String> = String::from_utf8_lossy(&listed.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    let tracked: HashSet<String> = git
        .stdout_lines(&["ls-tree", "-r", "--name-only", sha])?
        .into_iter()
        .collect();
    let template = template_for(ctx)?;
    let mut files = 0usize;
    for name in &names {
        let dest = dir.path().join(name);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let blob = git.run(&["show", &format!("{prefix}/{name}")])?;
        if name.ends_with(".md") {
            let text = String::from_utf8_lossy(&blob.stdout).to_string();
            fs::write(
                &dest,
                rewrite_page(&text, &tracked, &template, sha, identity),
            )?;
        } else {
            fs::write(&dest, &blob.stdout)?;
        }
        files += 1;
    }
    fs::write(
        dir.path().join(STAMP_FILE),
        stamp_json(sha, &identity.origin),
    )?;
    Ok(Built { dir, files })
}

fn template_for(ctx: &Context) -> Result<String> {
    if let Some(custom) = ctx.config()?.permalink_template.as_ref() {
        return Ok(custom.clone());
    }
    let host = ctx.identity()?.host.as_str();
    Ok(HOST_TEMPLATES
        .iter()
        .find(|(h, _)| *h == host)
        .map(|(_, t)| (*t).to_string())
        .unwrap_or_else(|| DEFAULT_TEMPLATE.to_string()))
}

pub(crate) fn rewrite_page(
    text: &str,
    tracked: &HashSet<String>,
    template: &str,
    sha: &str,
    identity: &crate::identity::RepoIdentity,
) -> String {
    let (front, body) = crate::frontmatter::split(text);
    let rewritten = rewrite_body(body, tracked, template, sha, identity);
    match front {
        Some(front) => format!("---\n{front}---\n{rewritten}"),
        None => rewritten,
    }
}

fn rewrite_body(
    body: &str,
    tracked: &HashSet<String>,
    template: &str,
    sha: &str,
    identity: &crate::identity::RepoIdentity,
) -> String {
    let mut out = String::with_capacity(body.len());
    let mut fence: Option<String> = None;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let is_fence = trimmed.starts_with("```") || trimmed.starts_with("~~~");
        if is_fence {
            let marker = if trimmed.starts_with("```") {
                "```"
            } else {
                "~~~"
            };
            fence = match &fence {
                Some(open) if open == marker => None,
                Some(open) => Some(open.clone()),
                None => Some(marker.to_string()),
            };
            out.push_str(line);
            continue;
        }
        if fence.is_some() {
            out.push_str(line);
            continue;
        }
        let Some(pointer) = POINTER.as_ref() else {
            out.push_str(line);
            continue;
        };
        let replaced = pointer.replace_all(line, |caps: &Captures<'_>| {
            let path = &caps["path"];
            if !tracked.contains(path) {
                return caps[0].to_string();
            }
            let a = &caps["a"];
            let anchor = match caps.name("b") {
                Some(b) => format!("{a}-L{}", b.as_str()),
                None => a.to_string(),
            };
            let url = template
                .replace("{host}", &identity.host)
                .replace("{owner}", &identity.owner)
                .replace("{repo}", &identity.repo)
                .replace("{sha}", sha)
                .replace("{path}", path)
                .replace("{line}", &anchor)
                .replace("{line_end}", caps.name("b").map_or("", |m| m.as_str()));
            let label = match caps.name("b") {
                Some(b) => format!("{path}:{a}-{}", b.as_str()),
                None => format!("{path}:{a}"),
            };
            format!("[{label}]({url})")
        });
        out.push_str(&replaced);
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::identity::from_origin_url;

    fn tracked() -> HashSet<String> {
        ["src/publish/sqs.py".to_string()].into_iter().collect()
    }

    fn github() -> crate::identity::RepoIdentity {
        from_origin_url("git@github.com:acme/ingest-api.git").unwrap()
    }

    #[test]
    fn s4_github_pointer_becomes_a_pinned_link() {
        let out = rewrite_page(
            "see `src/publish/sqs.py:57` now\n",
            &tracked(),
            HOST_TEMPLATES[0].1,
            "4f1c9a2",
            &github(),
        );
        assert_eq!(
            out,
            "see [src/publish/sqs.py:57](https://github.com/acme/ingest-api/blob/4f1c9a2/src/publish/sqs.py#L57) now\n"
        );
    }

    #[test]
    fn s4_range_pointer_uses_the_range_anchor() {
        let out = rewrite_page(
            "`src/publish/sqs.py:10-20`\n",
            &tracked(),
            HOST_TEMPLATES[0].1,
            "abc",
            &github(),
        );
        assert!(out.contains("#L10-L20"), "{out}");
        assert!(out.contains("[src/publish/sqs.py:10-20]"), "{out}");
    }

    #[test]
    fn s4_gitlab_template_differs() {
        let out = rewrite_page(
            "`src/publish/sqs.py:57`\n",
            &tracked(),
            HOST_TEMPLATES[1].1,
            "abc",
            &github(),
        );
        assert!(out.contains("/-/blob/abc/"), "{out}");
    }

    #[test]
    fn s4_pointer_outside_tree_untouched() {
        let out = rewrite_page(
            "`src/gone.py:1`\n",
            &tracked(),
            HOST_TEMPLATES[0].1,
            "abc",
            &github(),
        );
        assert_eq!(out, "`src/gone.py:1`\n");
    }

    #[test]
    fn s4_fenced_blocks_and_frontmatter_untouched() {
        let page = "---\nsite: src/publish/sqs.py:57\n---\n```\n`src/publish/sqs.py:57`\n```\n`src/publish/sqs.py:57`\n";
        let out = rewrite_page(page, &tracked(), HOST_TEMPLATES[0].1, "abc", &github());
        assert!(
            out.starts_with("---\nsite: src/publish/sqs.py:57\n---\n"),
            "{out}"
        );
        assert_eq!(out.matches("https://github.com").count(), 1, "{out}");
    }
}
