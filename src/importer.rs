//! Copies the docs folder at one commit, with pinned permalinks.

use std::collections::{BTreeSet, HashSet};
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

const LINK_PATTERN: &str = r#"\]\((?P<target>[^)\s]+)(?P<title>\s+"[^"]*")?\)"#;

static LINK: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(LINK_PATTERN).ok());

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct UnverifiedSite {
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) site: String,
}

#[derive(Debug)]
pub(crate) struct Built {
    pub(crate) dir: TempDir,
    pub(crate) files: usize,
    pub(crate) unverified: Vec<UnverifiedSite>,
    pub(crate) secrets: Vec<crate::secrets::SecretHit>,
}

/// Copies one unit's docs folder. `umbrella_of` is non-empty only for the root
/// index-of-indexes: its pages skip anything nested inside a target's docs dir
/// and its `00-index.md` has its links pointed at the sibling folders.
pub(crate) fn build(
    ctx: &Context,
    sha: &str,
    target: &crate::config::Target,
    umbrella_of: &[crate::config::Target],
) -> Result<Built> {
    let config = ctx.config()?;
    let identity = ctx.identity()?;
    let git = ctx.repo_git();
    let dir = tempfile::Builder::new()
        .prefix(".build-")
        .tempdir_in(ctx.quarry_dir())?;
    let prefix = format!("{sha}:{}", target.docs_dir);
    let listed = git.run_unchecked(&["ls-tree", "-r", "--name-only", &prefix])?;
    if !listed.status.success() {
        return Err(QuarryError::refusal(format!(
            "no {} at {sha}; run Capstone map first",
            target.docs_dir
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
    let base = pointer_base(&config.docs_dir, &target.docs_dir);
    let mut files = 0usize;
    let mut unverified: Vec<UnverifiedSite> = Vec::new();
    let mut secrets: Vec<crate::secrets::SecretHit> = Vec::new();
    for name in &names {
        if nested_in_a_target(&target.docs_dir, name, umbrella_of) {
            continue;
        }
        let dest = dir.path().join(name);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let blob = git.run(&["show", &format!("{prefix}/{name}")])?;
        let text = String::from_utf8_lossy(&blob.stdout);
        // Every copied file, markdown or not: an .env.example under the docs
        // folder is where a key leaks, and the patterns are ASCII, so a lossy
        // decode of a binary cannot invent a hit.
        for pattern in crate::secrets::scan(&text) {
            secrets.push(crate::secrets::SecretHit {
                file: format!("{}/{name}", target.docs_dir),
                pattern,
            });
        }
        if name.ends_with(".md") {
            unverified.extend(unverified_sites(name, &text, &tracked, base));
            let mut rendered = rewrite_page(&text, &tracked, &template, sha, identity, base);
            if !umbrella_of.is_empty() && name == "00-index.md" {
                rendered = rewrite_index_links(&rendered, &target.docs_dir, umbrella_of);
            }
            fs::write(&dest, rendered)?;
        } else {
            fs::write(&dest, &blob.stdout)?;
        }
        files += 1;
    }
    unverified.sort();
    unverified.dedup();
    fs::write(
        dir.path().join(STAMP_FILE),
        stamp_json(
            sha,
            &identity.origin,
            &target.docs_dir,
            &stamp_keys(&unverified),
        ),
    )?;
    Ok(Built {
        dir,
        files,
        unverified,
        secrets,
    })
}

/// The prefix a workspace's own `file:line` pointers are relative to, when its
/// docs dir is the root docs dir under some path.
pub(crate) fn pointer_base<'a>(root_docs_dir: &str, target_docs_dir: &'a str) -> Option<&'a str> {
    let root = crate::config::tidy_dir(root_docs_dir);
    let target = crate::config::tidy_dir(target_docs_dir);
    target
        .strip_suffix(&format!("/{root}"))
        .filter(|prefix| !prefix.is_empty())
}

fn nested_in_a_target(
    root_docs_dir: &str,
    name: &str,
    umbrella_of: &[crate::config::Target],
) -> bool {
    if umbrella_of.is_empty() {
        return false;
    }
    let full = format!("{}/{name}", crate::config::tidy_dir(root_docs_dir));
    umbrella_of.iter().any(|target| {
        let dir = crate::config::tidy_dir(&target.docs_dir);
        full == dir || full.starts_with(&format!("{dir}/"))
    })
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

// A prescriptive chapter names planned paths, so its sites are not checked;
// the Capstone script applies the same skip.
pub(crate) fn unverified_sites(
    path: &str,
    text: &str,
    tracked: &HashSet<String>,
    base: Option<&str>,
) -> Vec<UnverifiedSite> {
    let parsed = crate::frontmatter::parse(text);
    if parsed
        .fields
        .get("mode")
        .and_then(|v| v.as_str())
        .is_some_and(|mode| mode.trim().eq_ignore_ascii_case("prescriptive"))
    {
        return Vec::new();
    }
    let (edges, _) = crate::frontmatter::edges_of(path, &parsed.fields, &parsed.body);
    edges
        .into_iter()
        .filter_map(|edge| {
            let site = edge.site?;
            if is_tracked(tracked, crate::frontmatter::site_path(&site), base) {
                return None;
            }
            Some(UnverifiedSite {
                kind: edge.kind,
                name: edge.name,
                site,
            })
        })
        .collect()
}

pub(crate) fn unverified_note(site: &UnverifiedSite, sha: &str) -> String {
    format!(
        "site {} for {} {} is not in the tree at {}",
        site.site,
        site.kind,
        site.name,
        sha.chars().take(7).collect::<String>()
    )
}

pub(crate) fn stamp_keys(sites: &[UnverifiedSite]) -> Vec<String> {
    sites
        .iter()
        .map(|s| format!("{} {}", s.kind, s.name))
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

pub(crate) fn rewrite_page(
    text: &str,
    tracked: &HashSet<String>,
    template: &str,
    sha: &str,
    identity: &crate::identity::RepoIdentity,
    base: Option<&str>,
) -> String {
    let (front, body) = crate::frontmatter::split(text);
    let rewritten = rewrite_body(body, tracked, template, sha, identity, base);
    match front {
        Some(front) => format!("---\n{front}---\n{rewritten}"),
        None => rewritten,
    }
}

fn is_tracked(tracked: &HashSet<String>, path: &str, base: Option<&str>) -> bool {
    tracked.contains(path) || base.is_some_and(|b| tracked.contains(&format!("{b}/{path}")))
}

// Returns true when the line opens or closes a fence, updating the state.
fn fence_toggle(trimmed: &str, fence: &mut Option<String>) -> bool {
    let marker = if trimmed.starts_with("```") {
        "```"
    } else if trimmed.starts_with("~~~") {
        "~~~"
    } else {
        return false;
    };
    *fence = match fence.as_deref() {
        Some(open) if open == marker => None,
        Some(open) => Some(open.to_string()),
        None => Some(marker.to_string()),
    };
    true
}

fn rewrite_body(
    body: &str,
    tracked: &HashSet<String>,
    template: &str,
    sha: &str,
    identity: &crate::identity::RepoIdentity,
    base: Option<&str>,
) -> String {
    let mut out = String::with_capacity(body.len());
    let mut fence: Option<String> = None;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if fence_toggle(trimmed, &mut fence) {
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
            // A workspace chapter writes its pointers relative to the workspace,
            // so the URL takes the prefixed path while the label stays as written.
            let linked = if tracked.contains(path) {
                path.to_string()
            } else if let Some(prefixed) = base
                .map(|b| format!("{b}/{path}"))
                .filter(|p| tracked.contains(p))
            {
                prefixed
            } else {
                return caps[0].to_string();
            };
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
                .replace("{path}", &linked)
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

/// Points the root index-of-indexes at the sibling folders each target lands in.
/// Only inline links are rewritten; reference-style definitions are left alone.
pub(crate) fn rewrite_index_links(
    text: &str,
    root_docs_dir: &str,
    targets: &[crate::config::Target],
) -> String {
    let (front, body) = crate::frontmatter::split(text);
    let mut out = String::with_capacity(body.len());
    let mut fence: Option<String> = None;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if fence_toggle(trimmed, &mut fence) {
            out.push_str(line);
            continue;
        }
        if fence.is_some() {
            out.push_str(line);
            continue;
        }
        let Some(link) = LINK.as_ref() else {
            out.push_str(line);
            continue;
        };
        let replaced = link.replace_all(line, |caps: &Captures<'_>| {
            match rewrite_link_target(&caps["target"], root_docs_dir, targets) {
                Some(rewritten) => format!(
                    "]({rewritten}{})",
                    caps.name("title").map_or("", |m| m.as_str())
                ),
                None => caps[0].to_string(),
            }
        });
        out.push_str(&replaced);
    }
    match front {
        Some(front) => format!("---\n{front}---\n{out}"),
        None => out,
    }
}

fn rewrite_link_target(
    link: &str,
    root_docs_dir: &str,
    targets: &[crate::config::Target],
) -> Option<String> {
    if link.is_empty()
        || link.contains("://")
        || link.starts_with("mailto:")
        || link.starts_with('#')
        || link.starts_with('/')
    {
        return None;
    }
    let (path, fragment) = match link.find('#') {
        Some(at) => (&link[..at], &link[at..]),
        None => (link, ""),
    };
    if path.is_empty() {
        return None;
    }
    // The docs-dir-relative reading first, then the path as written from the
    // repo root: Capstone does not pin which of the two its index table uses.
    let mut candidates: Vec<String> = Vec::new();
    if let Some(resolved) = resolve_relative(root_docs_dir, path) {
        candidates.push(resolved);
    }
    candidates.push(path.to_string());
    let mut ordered: Vec<&crate::config::Target> = targets.iter().collect();
    ordered.sort_by_key(|t| std::cmp::Reverse(crate::config::tidy_dir(&t.docs_dir).len()));
    for candidate in &candidates {
        for target in &ordered {
            let dir = crate::config::tidy_dir(&target.docs_dir);
            let remainder = if candidate == dir {
                ""
            } else if let Some(rest) = candidate.strip_prefix(&format!("{dir}/")) {
                rest
            } else {
                continue;
            };
            return Some(format!("../{}/{remainder}{fragment}", target.name));
        }
    }
    None
}

/// `link` read against `base_dir`, with `.` and `..` folded away. `None` when it
/// climbs above the repo root.
fn resolve_relative(base_dir: &str, link: &str) -> Option<String> {
    let mut stack: Vec<&str> = crate::config::tidy_dir(base_dir)
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    for part in link.split('/') {
        match part {
            ".." => {
                stack.pop()?;
            }
            "." | "" => {}
            other => stack.push(other),
        }
    }
    Some(stack.join("/"))
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
            None,
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
            None,
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
            None,
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
            None,
        );
        assert_eq!(out, "`src/gone.py:1`\n");
    }

    #[test]
    fn s4_fenced_blocks_and_frontmatter_untouched() {
        let page = "---\nsite: src/publish/sqs.py:57\n---\n```\n`src/publish/sqs.py:57`\n```\n`src/publish/sqs.py:57`\n";
        let out = rewrite_page(
            page,
            &tracked(),
            HOST_TEMPLATES[0].1,
            "abc",
            &github(),
            None,
        );
        assert!(
            out.starts_with("---\nsite: src/publish/sqs.py:57\n---\n"),
            "{out}"
        );
        assert_eq!(out.matches("https://github.com").count(), 1, "{out}");
    }

    fn table_page(site: &str) -> String {
        format!(
            "---\ngenerated_date: 2026-09-04\n---\n\n## Produces\n\n| Kind | Name | To | Site |\n|---|---|---|---|\n| sqs | file-ingest | record-store | `{site}` |\n"
        )
    }

    #[test]
    fn s18_an_untracked_site_is_reported_with_its_edge() {
        let found = unverified_sites(
            "09-interfaces.md",
            &table_page("src/gone.py:12"),
            &tracked(),
            None,
        );
        assert_eq!(
            found,
            vec![UnverifiedSite {
                kind: "sqs".to_string(),
                name: "file-ingest".to_string(),
                site: "src/gone.py:12".to_string(),
            }]
        );
        assert_eq!(
            unverified_note(&found[0], "4f1c9a2abcdef"),
            "site src/gone.py:12 for sqs file-ingest is not in the tree at 4f1c9a2"
        );
    }

    #[test]
    fn s18_a_tracked_site_with_a_range_is_verified() {
        let found = unverified_sites(
            "09-interfaces.md",
            &table_page("src/publish/sqs.py:10-20"),
            &tracked(),
            None,
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn s18_a_site_without_a_line_is_verified_by_its_path() {
        let found = unverified_sites(
            "09-interfaces.md",
            &table_page("src/publish/sqs.py"),
            &tracked(),
            None,
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn s18_stamp_keys_are_sorted_and_deduped() {
        let site = |kind: &str, name: &str, site: &str| UnverifiedSite {
            kind: kind.to_string(),
            name: name.to_string(),
            site: site.to_string(),
        };
        let sites = [
            site("sqs", "file-ingest", "a.rs:1"),
            site("sqs", "file-ingest", "b.rs:2"),
            site("http", "GET /x", "c.rs:3"),
        ];
        assert_eq!(stamp_keys(&sites), vec!["http GET /x", "sqs file-ingest"]);
    }

    #[test]
    fn s18_sites_off_the_interfaces_page_come_from_frontmatter_only() {
        let from_table = unverified_sites(
            "01-architecture.md",
            &table_page("src/gone.py:12"),
            &tracked(),
            None,
        );
        assert!(from_table.is_empty(), "{from_table:?}");
        let page = "---\nproduces:\n  - kind: sqs\n    name: x\n    to: record-store\n    site: src/gone.py:1\n---\n\nbody\n";
        let from_front = unverified_sites("01-architecture.md", page, &tracked(), None);
        assert_eq!(from_front.len(), 1, "{from_front:?}");
        assert_eq!(from_front[0].site, "src/gone.py:1");
    }

    #[test]
    fn s18_a_prescriptive_page_skips_site_checks() {
        let page = format!(
            "---\ngenerated_date: 2026-09-04\nmode: prescriptive\n---\n{}",
            crate::frontmatter::parse(&table_page("src/gone.py:12")).body
        );
        let found = unverified_sites("09-interfaces.md", &page, &tracked(), None);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn s18_prescriptive_mode_is_matched_case_insensitively() {
        let body = crate::frontmatter::parse(&table_page("src/gone.py:12")).body;
        for mode in ["Prescriptive", "PRESCRIPTIVE", "  prescriptive  "] {
            let page = format!("---\ngenerated_date: 2026-09-04\nmode: \"{mode}\"\n---\n{body}");
            let found = unverified_sites("09-interfaces.md", &page, &tracked(), None);
            assert!(found.is_empty(), "{mode}: {found:?}");
        }
    }

    fn workspaces() -> Vec<crate::config::Target> {
        vec![
            crate::config::Target {
                name: "billing".to_string(),
                docs_dir: "services/billing/docs/capstone".to_string(),
            },
            crate::config::Target {
                name: "orders".to_string(),
                docs_dir: "services/orders/docs/capstone".to_string(),
            },
        ]
    }

    fn relink(line: &str) -> String {
        rewrite_index_links(line, "docs/capstone", &workspaces())
    }

    #[test]
    fn s17_umbrella_index_links_into_a_target_are_rewritten() {
        assert_eq!(
            relink("| billing | [billing](../../services/billing/docs/capstone/00-index.md) |\n"),
            "| billing | [billing](../billing/00-index.md) |\n"
        );
    }

    #[test]
    fn s17_fragments_and_titles_survive_the_rewrite() {
        assert_eq!(
            relink("[b](../../services/billing/docs/capstone/00-index.md#overview \"Billing\")\n"),
            "[b](../billing/00-index.md#overview \"Billing\")\n"
        );
    }

    #[test]
    fn s17_repo_root_relative_links_are_also_rewritten() {
        assert_eq!(
            relink("[o](services/orders/docs/capstone/00-index.md)\n"),
            "[o](../orders/00-index.md)\n"
        );
    }

    #[test]
    fn s17_links_outside_every_target_are_untouched() {
        for line in [
            "[a](01-architecture.md)\n",
            "[r](../../README.md)\n",
            "[x](https://x)\n",
            "[t](#top)\n",
            "[abs](/abs.md)\n",
        ] {
            assert_eq!(relink(line), line);
        }
    }

    #[test]
    fn s17_a_link_climbing_out_of_the_repo_is_untouched() {
        let line = "[e](../../../elsewhere/docs/capstone/00-index.md)\n";
        assert_eq!(relink(line), line);
    }

    #[test]
    fn s17_links_in_fenced_blocks_are_untouched() {
        let page = "```\n[b](../../services/billing/docs/capstone/00-index.md)\n```\n[b](../../services/billing/docs/capstone/00-index.md)\n";
        assert_eq!(
            relink(page),
            "```\n[b](../../services/billing/docs/capstone/00-index.md)\n```\n[b](../billing/00-index.md)\n"
        );
    }

    // config::validate_targets refuses a target docs dir inside another target's,
    // so this layout never reaches build(); the longest-first order is kept as
    // defence in depth for the link rewrite alone.
    #[test]
    fn s17_a_nested_target_wins_by_longest_docs_dir() {
        let nested = vec![
            crate::config::Target {
                name: "a".to_string(),
                docs_dir: "docs/capstone/a".to_string(),
            },
            crate::config::Target {
                name: "b".to_string(),
                docs_dir: "docs/capstone/a/b".to_string(),
            },
        ];
        assert_eq!(
            rewrite_index_links("[b](a/b/00-index.md)\n", "docs/capstone", &nested),
            "[b](../b/00-index.md)\n"
        );
        assert_eq!(
            rewrite_index_links("[a](a/00-index.md)\n", "docs/capstone", &nested),
            "[a](../a/00-index.md)\n"
        );
    }

    #[test]
    fn s17_frontmatter_is_kept_by_the_link_rewrite() {
        let page = "---\ngenerated_date: 2026-09-04\n---\n\n[b](../../services/billing/docs/capstone/00-index.md)\n";
        let out = relink(page);
        assert!(
            out.starts_with("---\ngenerated_date: 2026-09-04\n---\n"),
            "{out}"
        );
        assert!(out.ends_with("[b](../billing/00-index.md)\n"), "{out}");
    }

    #[test]
    fn s17_a_target_folder_link_with_no_remainder_still_resolves() {
        assert_eq!(
            relink("[b](../../services/billing/docs/capstone)\n"),
            "[b](../billing/)\n"
        );
    }

    #[test]
    fn s17_workspace_relative_pointers_resolve_under_the_base() {
        let tracked: HashSet<String> = ["services/billing/src/x.rs".to_string()]
            .into_iter()
            .collect();
        let out = rewrite_page(
            "see `src/x.rs:3`\n",
            &tracked,
            HOST_TEMPLATES[0].1,
            "4f1c9a2",
            &github(),
            Some("services/billing"),
        );
        assert_eq!(
            out,
            "see [src/x.rs:3](https://github.com/acme/ingest-api/blob/4f1c9a2/services/billing/src/x.rs#L3)\n"
        );
    }

    #[test]
    fn s17_workspace_relative_sites_verify_under_the_base() {
        let tracked: HashSet<String> = ["services/billing/src/publish.py".to_string()]
            .into_iter()
            .collect();
        let page = table_page("src/publish.py:1");
        assert!(
            unverified_sites(
                "09-interfaces.md",
                &page,
                &tracked,
                Some("services/billing")
            )
            .is_empty()
        );
        assert_eq!(
            unverified_sites("09-interfaces.md", &page, &tracked, None).len(),
            1
        );
    }

    #[test]
    fn s17_pointer_base_is_the_prefix_above_the_root_docs_dir() {
        assert_eq!(
            pointer_base("docs/capstone", "services/billing/docs/capstone"),
            Some("services/billing")
        );
        assert_eq!(pointer_base("docs/capstone", "docs/capstone"), None);
        assert_eq!(pointer_base("docs/capstone", "services/billing/docs"), None);
    }

    #[test]
    fn s17_resolve_relative_normalises_dots() {
        assert_eq!(
            resolve_relative("docs/capstone", "../../a/./b"),
            Some("a/b".to_string())
        );
        assert_eq!(resolve_relative("docs", "../../x"), None);
    }

    #[test]
    fn s17_a_nested_target_page_is_skipped_by_the_umbrella() {
        let nested = vec![crate::config::Target {
            name: "billing".to_string(),
            docs_dir: "docs/capstone/billing".to_string(),
        }];
        assert!(nested_in_a_target(
            "docs/capstone",
            "billing/00-index.md",
            &nested
        ));
        assert!(!nested_in_a_target("docs/capstone", "00-index.md", &nested));
        assert!(!nested_in_a_target(
            "docs/capstone",
            "billing/00-index.md",
            &[]
        ));
    }
}
