//! The derived SQLite index over the docs repo clone.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::context::Context;
use crate::docsrepo;
use crate::errors::{QuarryError, Result};
use crate::frontmatter;

pub(crate) const SCHEMA_VERSION: i64 = 2;

type EdgeKey = (String, String, String, String);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Declaration {
    owner: String,
    other: String,
    kind: String,
    name: String,
    produces: bool,
    via: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct EdgeSides {
    by_producer: bool,
    by_consumer: bool,
    via: BTreeSet<String>,
    declared_as: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NameRegistry {
    pub(crate) names: BTreeSet<String>,
    pub(crate) folded: BTreeMap<String, String>,
    pub(crate) aliases: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Unresolved {
    pub(crate) declared: String,
    pub(crate) via: Vec<String>,
    pub(crate) nearest: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Publication {
    pub(crate) repo: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) via: Vec<String>,
}

pub(crate) const MAX_PAGE_BYTES: u64 = 5 * 1024 * 1024;

const SCHEMA: &str = "
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE repos (
  name TEXT PRIMARY KEY,
  commit_sha TEXT,
  origin TEXT,
  newest_generated_date TEXT,
  pages INTEGER NOT NULL,
  produces INTEGER NOT NULL DEFAULT 0,
  consumes INTEGER NOT NULL DEFAULT 0,
  known_as TEXT NOT NULL DEFAULT '[]'
);
CREATE TABLE pages (
  repo TEXT NOT NULL,
  path TEXT NOT NULL,
  frontmatter TEXT NOT NULL,
  generated_date TEXT,
  PRIMARY KEY (repo, path)
);
CREATE TABLE edges (
  from_repo TEXT NOT NULL,
  to_repo TEXT NOT NULL,
  kind TEXT NOT NULL,
  name TEXT NOT NULL,
  declared_by TEXT NOT NULL CHECK (declared_by IN ('producer','consumer','both')),
  via TEXT NOT NULL,
  missing INTEGER NOT NULL DEFAULT 0,
  site_unverified INTEGER NOT NULL DEFAULT 0,
  as_declared TEXT,
  PRIMARY KEY (from_repo, to_repo, kind, name)
);
CREATE INDEX edges_to ON edges (to_repo);
CREATE TABLE aliases (alias TEXT PRIMARY KEY, repo TEXT NOT NULL);
CREATE TABLE publications (
  repo TEXT NOT NULL,
  kind TEXT NOT NULL,
  name TEXT NOT NULL,
  via TEXT NOT NULL,
  PRIMARY KEY (repo, kind, name)
);
CREATE VIRTUAL TABLE sections USING fts5 (
  repo UNINDEXED, path UNINDEXED, heading, normalized UNINDEXED, level UNINDEXED, body,
  tokenize = 'unicode61'
);
";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RepoRow {
    pub(crate) name: String,
    pub(crate) commit: Option<String>,
    pub(crate) origin: Option<String>,
    pub(crate) newest_generated_date: Option<String>,
    pub(crate) pages: u32,
    pub(crate) produces: u32,
    pub(crate) consumes: u32,
    pub(crate) known_as: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct Edge {
    pub(crate) from_repo: String,
    pub(crate) to_repo: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) declared_by: String,
    pub(crate) via: Vec<String>,
    pub(crate) missing: bool,
    pub(crate) site_unverified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) as_declared: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PageRow {
    pub(crate) repo: String,
    pub(crate) path: String,
    pub(crate) generated_date: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RepoScan {
    pub(crate) commit: Option<String>,
    pub(crate) origin: Option<String>,
    pub(crate) newest_generated_date: Option<String>,
    pub(crate) pages: u32,
    pub(crate) produces: u32,
    pub(crate) consumes: u32,
    pub(crate) known_as: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct RebuildReport {
    pub(crate) repos: u32,
    pub(crate) pages: u32,
    pub(crate) edges: u32,
    pub(crate) warnings: Vec<String>,
    pub(crate) rebuilt: bool,
    pub(crate) unresolved: Vec<Unresolved>,
}

pub(crate) struct Index {
    pub(crate) connection: Connection,
    pub(crate) built_at_commit: String,
    pub(crate) synced_at: Option<String>,
    pub(crate) report: RebuildReport,
}

pub(crate) fn open_current(ctx: &Context) -> Result<Index> {
    open_with(ctx, false)
}

pub(crate) fn open_with(ctx: &Context, force: bool) -> Result<Index> {
    let clone = ctx.require_clone()?;
    let head = head_of(ctx)?;
    let path = ctx.index_path();
    let mut report = RebuildReport::default();
    if force || !is_current(&path, &head) {
        report = rebuild(ctx, &clone, &head)?;
    }
    let connection = Connection::open(&path)?;
    let synced_at = read_meta(&connection, "synced_at")?;
    Ok(Index {
        connection,
        built_at_commit: head,
        synced_at,
        report,
    })
}

fn is_current(path: &Path, head: &str) -> bool {
    if !path.exists() {
        return false;
    }
    let Ok(connection) = Connection::open(path) else {
        return false;
    };
    let version = read_meta(&connection, "schema_version")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok());
    let built = read_meta(&connection, "built_at_commit").ok().flatten();
    version == Some(SCHEMA_VERSION) && built.as_deref() == Some(head)
}

fn read_meta(connection: &Connection, key: &str) -> Result<Option<String>> {
    let mut statement = match connection.prepare("SELECT value FROM meta WHERE key = ?1") {
        Ok(statement) => statement,
        Err(_) => return Ok(None),
    };
    let mut rows = statement.query([key])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub(crate) fn head_of(ctx: &Context) -> Result<String> {
    let git = ctx.clone_git()?;
    match git.run_unchecked(&["rev-parse", "HEAD"])? {
        out if out.status.success() => Ok(String::from_utf8_lossy(&out.stdout).trim().to_string()),
        _ => Ok("empty".to_string()),
    }
}

pub(crate) fn rebuild(ctx: &Context, clone: &Path, head: &str) -> Result<RebuildReport> {
    let final_path = ctx.index_path();
    let tmp_path = final_path.with_extension("sqlite.tmp");
    let _ = fs::remove_file(&tmp_path);
    let previous_sync = if final_path.exists() {
        Connection::open(&final_path)
            .ok()
            .and_then(|c| read_meta(&c, "synced_at").ok().flatten())
    } else {
        None
    };
    let mut report = RebuildReport {
        rebuilt: true,
        ..RebuildReport::default()
    };
    {
        let mut connection = Connection::open(&tmp_path)?;
        connection.execute_batch(SCHEMA)?;
        let transaction = connection.transaction()?;
        let mut declarations: Vec<Declaration> = Vec::new();
        let mut names: BTreeSet<String> = BTreeSet::new();
        let mut unverified: BTreeSet<(String, String)> = BTreeSet::new();
        let mut claims: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut publications_acc: BTreeMap<(String, String, String), BTreeSet<String>> =
            BTreeMap::new();
        for dir in docsrepo::read_repo_dirs(clone)? {
            let name = dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            names.insert(name.clone());
            let mut scan = RepoScan::default();
            let mut known_as: BTreeSet<String> = BTreeSet::new();
            let stamp_path = dir.join(docsrepo::STAMP_FILE);
            if let Ok(text) = fs::read_to_string(&stamp_path)
                && let Ok(stamp) = serde_json::from_str::<docsrepo::Stamp>(&text)
            {
                scan.commit = Some(stamp.commit);
                scan.origin = stamp.origin;
                for key in stamp.unverified {
                    unverified.insert((name.clone(), key));
                }
            }
            for page in markdown_files(&dir)? {
                let relative = page
                    .strip_prefix(&dir)
                    .unwrap_or(&page)
                    .to_string_lossy()
                    .replace('\\', "/");
                let size = fs::metadata(&page).map(|m| m.len()).unwrap_or(0);
                let text = fs::read_to_string(&page).unwrap_or_default();
                let parsed = frontmatter::parse(&text);
                if let Some(warning) = &parsed.warning {
                    report
                        .warnings
                        .push(format!("{name}/{relative}: {warning}"));
                }
                let (aliases, alias_warnings) = frontmatter::known_as_of(&parsed.fields);
                for warning in alias_warnings {
                    report
                        .warnings
                        .push(format!("{name}/{relative}: {warning}"));
                }
                known_as.extend(aliases);
                let generated_date = parsed
                    .fields
                    .get("generated_date")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                if let Some(date) = &generated_date
                    && scan.newest_generated_date.as_deref().unwrap_or("") < date.as_str()
                {
                    scan.newest_generated_date = Some(date.clone());
                }
                let fields_json = serde_json::to_string(&parsed.fields)?;
                transaction.execute(
                    "INSERT OR REPLACE INTO pages (repo, path, frontmatter, generated_date) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![name, relative, fields_json, generated_date],
                )?;
                scan.pages += 1;
                report.pages += 1;
                if size <= MAX_PAGE_BYTES {
                    for section in frontmatter::sections_of(&parsed.body) {
                        transaction.execute(
                            "INSERT INTO sections (repo, path, heading, normalized, level, body) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            rusqlite::params![name, relative, section.heading, section.normalized, section.level as i64, section.body],
                        )?;
                    }
                } else {
                    report
                        .warnings
                        .push(format!("{name}/{relative}: over 5 MB, body not indexed"));
                }
                let (edges, warnings) =
                    frontmatter::edges_of(&relative, &parsed.fields, &parsed.body);
                for warning in warnings {
                    report
                        .warnings
                        .push(format!("{name}/{relative}: {warning}"));
                }
                for edge in edges {
                    let via = format!("{name}/{relative}");
                    if edge.produces {
                        scan.produces += 1;
                    } else {
                        scan.consumes += 1;
                    }
                    if edge.produces && frontmatter::is_unknown_target(&edge.other) {
                        publications_acc
                            .entry((name.clone(), edge.kind, edge.name))
                            .or_default()
                            .insert(via);
                        continue;
                    }
                    declarations.push(Declaration {
                        owner: name.clone(),
                        other: edge.other,
                        kind: edge.kind,
                        name: edge.name,
                        produces: edge.produces,
                        via,
                    });
                }
            }
            for alias in &known_as {
                claims
                    .entry(alias.to_lowercase())
                    .or_default()
                    .insert(name.clone());
            }
            scan.known_as = known_as.into_iter().collect();
            transaction.execute(
                "INSERT OR REPLACE INTO repos (name, commit_sha, origin, newest_generated_date, pages, produces, consumes, known_as) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![name, scan.commit, scan.origin, scan.newest_generated_date, scan.pages, scan.produces, scan.consumes, serde_json::to_string(&scan.known_as)?],
            )?;
            report.repos += 1;
        }
        let (registry, alias_warnings) = build_registry(&names, &claims);
        report.warnings.extend(alias_warnings);
        let mut unresolved: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut merged: BTreeMap<EdgeKey, EdgeSides> = BTreeMap::new();
        for declaration in declarations {
            let resolved = resolve_name(&registry, &declaration.other);
            let other = match &resolved {
                Some(name) => name.clone(),
                None => {
                    if !frontmatter::is_unknown_target(&declaration.other) {
                        unresolved
                            .entry(declaration.other.clone())
                            .or_default()
                            .insert(declaration.via.clone());
                    }
                    declaration.other.clone()
                }
            };
            let (from, to) = if declaration.produces {
                (declaration.owner.clone(), other)
            } else {
                (other, declaration.owner.clone())
            };
            let entry = merged
                .entry((from, to, declaration.kind, declaration.name))
                .or_default();
            if declaration.produces {
                entry.by_producer = true;
            } else {
                entry.by_consumer = true;
            }
            entry.via.insert(declaration.via);
            if resolved.as_deref().is_some_and(|r| r != declaration.other) {
                entry.declared_as.insert(declaration.other);
            }
        }
        for ((from, to, kind, name), sides) in merged {
            let declared_by = match (sides.by_producer, sides.by_consumer) {
                (true, true) => "both",
                (true, false) => "producer",
                _ => "consumer",
            };
            let missing = !names.contains(&from) || !names.contains(&to);
            let key = format!("{kind} {name}");
            let site_unverified = unverified.contains(&(from.clone(), key.clone()))
                || unverified.contains(&(to.clone(), key));
            let via_json = serde_json::to_string(&sides.via.iter().collect::<Vec<_>>())?;
            let as_declared = if sides.declared_as.is_empty() {
                None
            } else {
                Some(
                    sides
                        .declared_as
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            };
            transaction.execute(
                "INSERT OR REPLACE INTO edges (from_repo, to_repo, kind, name, declared_by, via, missing, site_unverified, as_declared) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![from, to, kind, name, declared_by, via_json, missing as i64, site_unverified as i64, as_declared],
            )?;
            report.edges += 1;
        }
        for ((repo, kind, name), via) in publications_acc {
            transaction.execute(
                "INSERT INTO publications (repo, kind, name, via) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    repo,
                    kind,
                    name,
                    serde_json::to_string(&via.iter().collect::<Vec<_>>())?
                ],
            )?;
        }
        for (alias, repo) in &registry.aliases {
            transaction.execute(
                "INSERT INTO aliases (alias, repo) VALUES (?1, ?2)",
                rusqlite::params![alias, repo],
            )?;
        }
        report.unresolved = unresolved
            .into_iter()
            .map(|(declared, via)| Unresolved {
                nearest: nearest_names(&registry, &declared),
                via: via.into_iter().collect(),
                declared,
            })
            .collect();
        transaction.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('built_at_commit', ?1)",
            [head],
        )?;
        transaction.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', ?1)",
            [SCHEMA_VERSION.to_string()],
        )?;
        if let Some(synced) = previous_sync {
            transaction.execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES ('synced_at', ?1)",
                [synced],
            )?;
        }
        transaction.commit()?;
    }
    fs::rename(&tmp_path, &final_path)?;
    Ok(report)
}

pub(crate) fn set_synced_at(ctx: &Context, when: &str) -> Result<()> {
    let path = ctx.index_path();
    if !path.exists() {
        return Ok(());
    }
    let connection = Connection::open(&path)?;
    connection.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES ('synced_at', ?1)",
        [when],
    )?;
    Ok(())
}

pub(crate) fn scan_repo_dir(dir: &Path) -> Result<RepoScan> {
    let mut scan = RepoScan::default();
    if let Ok(text) = fs::read_to_string(dir.join(docsrepo::STAMP_FILE))
        && let Ok(stamp) = serde_json::from_str::<docsrepo::Stamp>(&text)
    {
        scan.commit = Some(stamp.commit);
        scan.origin = stamp.origin;
    }
    for page in markdown_files(dir)? {
        let relative = page
            .strip_prefix(dir)
            .unwrap_or(&page)
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&page).unwrap_or_default();
        let parsed = frontmatter::parse(&text);
        scan.pages += 1;
        if let Some(date) = parsed.fields.get("generated_date").and_then(|v| v.as_str())
            && scan.newest_generated_date.as_deref().unwrap_or("") < date
        {
            scan.newest_generated_date = Some(date.to_string());
        }
        let (edges, _) = frontmatter::edges_of(&relative, &parsed.fields, &parsed.body);
        for edge in edges {
            if edge.produces {
                scan.produces += 1;
            } else {
                scan.consumes += 1;
            }
        }
    }
    Ok(scan)
}

pub(crate) fn markdown_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(QuarryError::from(e)),
        };
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == ".git") {
                    continue;
                }
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "md") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

// Folder names win over aliases. A lowercase key two folders share is left
// out of `folded` (exact match still works for both); an alias two repos
// claim, or one that is another repo's folder name, is dropped with a warning.
pub(crate) fn build_registry(
    names: &BTreeSet<String>,
    claims: &BTreeMap<String, BTreeSet<String>>,
) -> (NameRegistry, Vec<String>) {
    let mut warnings = Vec::new();
    let mut by_lower: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in names {
        by_lower
            .entry(name.to_lowercase())
            .or_default()
            .push(name.clone());
    }
    let folded: BTreeMap<String, String> = by_lower
        .iter()
        .filter(|(_, owners)| owners.len() == 1)
        .map(|(lower, owners)| (lower.clone(), owners[0].clone()))
        .collect();
    let mut aliases: BTreeMap<String, String> = BTreeMap::new();
    for (alias, claimants) in claims {
        let remaining: BTreeSet<String> = claimants
            .iter()
            .filter(|c| c.to_lowercase() != *alias)
            .cloned()
            .collect();
        if remaining.is_empty() {
            continue;
        }
        if let Some(owners) = by_lower.get(alias) {
            let owner = owners.first().cloned().unwrap_or_default();
            for claimant in &remaining {
                warnings.push(format!(
                    "alias {alias} claimed by {claimant} is repo {owner}'s name; ignored"
                ));
            }
            continue;
        }
        if remaining.len() > 1 {
            warnings.push(format!(
                "alias {alias} claimed by {}; ignored",
                join_claimants(&remaining)
            ));
            continue;
        }
        if let Some(claimant) = remaining.into_iter().next() {
            aliases.insert(alias.clone(), claimant);
        }
    }
    (
        NameRegistry {
            names: names.clone(),
            folded,
            aliases,
        },
        warnings,
    )
}

fn join_claimants(claimants: &BTreeSet<String>) -> String {
    let list: Vec<&str> = claimants.iter().map(String::as_str).collect();
    match list.split_last() {
        None => String::new(),
        Some((last, [])) => (*last).to_string(),
        Some((last, head)) => format!("{} and {last}", head.join(", ")),
    }
}

pub(crate) fn resolve_name(registry: &NameRegistry, declared: &str) -> Option<String> {
    if registry.names.contains(declared) {
        return Some(declared.to_string());
    }
    let lower = declared.to_lowercase();
    if let Some(name) = registry.folded.get(&lower) {
        return Some(name.clone());
    }
    registry.aliases.get(&lower).cloned()
}

// Same scorer shape as `query::nearest`: longest shared prefix first, ties
// alphabetical, at most three. Aliases enter as their lowercased usable keys
// so every suggestion is a string that resolves.
pub(crate) fn nearest_names(registry: &NameRegistry, declared: &str) -> Vec<String> {
    let lower = declared.to_lowercase();
    let mut scored: Vec<(usize, String)> = registry
        .names
        .iter()
        .cloned()
        .chain(registry.aliases.keys().cloned())
        .map(|candidate| {
            let shared = candidate
                .to_lowercase()
                .chars()
                .zip(lower.chars())
                .take_while(|(a, b)| a == b)
                .count();
            (shared, candidate)
        })
        .filter(|(shared, _)| *shared > 0)
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.dedup_by(|a, b| a.1 == b.1);
    scored.into_iter().take(3).map(|(_, s)| s).collect()
}

impl Index {
    pub(crate) fn repos(&self) -> Result<Vec<RepoRow>> {
        let mut statement = self.connection.prepare(
            "SELECT name, commit_sha, origin, newest_generated_date, pages, produces, consumes, known_as FROM repos ORDER BY name",
        )?;
        let rows = statement.query_map([], |row| {
            let known_as: String = row.get(7)?;
            Ok(RepoRow {
                name: row.get(0)?,
                commit: row.get(1)?,
                origin: row.get(2)?,
                newest_generated_date: row.get(3)?,
                pages: row.get(4)?,
                produces: row.get(5)?,
                consumes: row.get(6)?,
                known_as: serde_json::from_str(&known_as).unwrap_or_default(),
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub(crate) fn has_repo(&self, repo: &str) -> Result<bool> {
        let mut statement = self
            .connection
            .prepare("SELECT 1 FROM repos WHERE name = ?1")?;
        Ok(statement.exists([repo])?)
    }

    pub(crate) fn repo(&self, repo: &str) -> Result<Option<RepoRow>> {
        Ok(self.repos()?.into_iter().find(|r| r.name == repo))
    }

    pub(crate) fn pages(&self, repo: &str) -> Result<Vec<PageRow>> {
        let mut statement = self
            .connection
            .prepare("SELECT path, generated_date FROM pages WHERE repo = ?1 ORDER BY path")?;
        let rows = statement.query_map([repo], |row| {
            Ok(PageRow {
                repo: repo.to_string(),
                path: row.get(0)?,
                generated_date: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub(crate) fn edges(&self, repo: &str, downstream: bool) -> Result<Vec<Edge>> {
        let sql = if downstream {
            "SELECT from_repo, to_repo, kind, name, declared_by, via, missing, site_unverified, as_declared FROM edges WHERE from_repo = ?1 ORDER BY to_repo, kind, name"
        } else {
            "SELECT from_repo, to_repo, kind, name, declared_by, via, missing, site_unverified, as_declared FROM edges WHERE to_repo = ?1 ORDER BY from_repo, kind, name"
        };
        let mut statement = self.connection.prepare(sql)?;
        let rows = statement.query_map([repo], row_to_edge)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub(crate) fn edges_touching(&self, repo: &str) -> Result<Vec<Edge>> {
        let mut statement = self.connection.prepare(
            "SELECT from_repo, to_repo, kind, name, declared_by, via, missing, site_unverified, as_declared FROM edges WHERE from_repo = ?1 OR to_repo = ?1 ORDER BY from_repo, to_repo, kind, name",
        )?;
        let rows = statement.query_map([repo], row_to_edge)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub(crate) fn repo_edges(&self, repo: &str) -> Result<(Vec<Edge>, Vec<Edge>)> {
        Ok((self.edges(repo, true)?, self.edges(repo, false)?))
    }

    pub(crate) fn publications(&self, repo: &str) -> Result<Vec<Publication>> {
        let mut statement = self.connection.prepare(
            "SELECT kind, name, via FROM publications WHERE repo = ?1 ORDER BY kind, name",
        )?;
        let rows = statement.query_map([repo], |row| {
            let via: String = row.get(2)?;
            Ok(Publication {
                repo: repo.to_string(),
                kind: row.get(0)?,
                name: row.get(1)?,
                via: serde_json::from_str(&via).unwrap_or_default(),
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}

fn row_to_edge(row: &rusqlite::Row<'_>) -> rusqlite::Result<Edge> {
    let via: String = row.get(5)?;
    Ok(Edge {
        from_repo: row.get(0)?,
        to_repo: row.get(1)?,
        kind: row.get(2)?,
        name: row.get(3)?,
        declared_by: row.get(4)?,
        via: serde_json::from_str(&via).unwrap_or_default(),
        missing: row.get::<_, i64>(6)? != 0,
        site_unverified: row.get::<_, i64>(7)? != 0,
        as_declared: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use std::collections::{BTreeMap, BTreeSet};

    use rusqlite::Connection;

    use super::{build_registry, nearest_names, resolve_name};

    fn names(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    fn claims(list: &[(&str, &[&str])]) -> BTreeMap<String, BTreeSet<String>> {
        list.iter()
            .map(|(alias, claimants)| ((*alias).to_string(), names(claimants)))
            .collect()
    }

    #[test]
    fn s15_resolution_order_is_exact_then_case_then_alias() {
        let (registry, warnings) = build_registry(
            &names(&["record-store", "Records"]),
            &claims(&[("records-svc", &["record-store"])]),
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            resolve_name(&registry, "record-store").as_deref(),
            Some("record-store")
        );
        assert_eq!(
            resolve_name(&registry, "Record-Store").as_deref(),
            Some("record-store")
        );
        assert_eq!(
            resolve_name(&registry, "RECORDS-SVC").as_deref(),
            Some("record-store")
        );
        assert_eq!(
            resolve_name(&registry, "records").as_deref(),
            Some("Records")
        );
        assert_eq!(resolve_name(&registry, "nope"), None);
    }

    #[test]
    fn s15_an_alias_claimed_twice_is_dropped_with_a_warning() {
        let (registry, warnings) =
            build_registry(&names(&["a", "b"]), &claims(&[("shared", &["a", "b"])]));
        assert!(registry.aliases.is_empty(), "{:?}", registry.aliases);
        assert_eq!(warnings, ["alias shared claimed by a and b; ignored"]);
    }

    #[test]
    fn s15_an_alias_equal_to_a_repo_name_is_dropped_with_a_warning() {
        let (registry, warnings) = build_registry(&names(&["a", "b"]), &claims(&[("b", &["a"])]));
        assert!(registry.aliases.is_empty(), "{:?}", registry.aliases);
        assert_eq!(warnings, ["alias b claimed by a is repo b's name; ignored"]);
        let (registry, warnings) = build_registry(&names(&["a", "b"]), &claims(&[("a", &["a"])]));
        assert!(registry.aliases.is_empty(), "{:?}", registry.aliases);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn s15_nearest_names_prefer_the_longest_prefix() {
        let (registry, _) = build_registry(
            &names(&[
                "record-store",
                "report-builder",
                "ingest-api",
                "records-api",
            ]),
            &claims(&[("records.internal", &["record-store"])]),
        );
        // Shared prefixes with `records-svc`: `records-api` 8 (`records-`),
        // `records.internal` 7 (`records`), `record-store` 6 (`record`); no tie.
        assert_eq!(
            nearest_names(&registry, "records-svc"),
            ["records-api", "records.internal", "record-store"]
        );
        assert!(nearest_names(&registry, "zzz").is_empty());
    }

    #[test]
    fn s5_bundled_sqlite_has_fts5() {
        let connection = Connection::open_in_memory().unwrap();
        let mut statement = connection.prepare("PRAGMA compile_options").unwrap();
        let options: Vec<String> = statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .filter_map(std::result::Result::ok)
            .collect();
        assert!(
            options.iter().any(|o| o.contains("ENABLE_FTS5")),
            "{options:?}"
        );
    }
}
