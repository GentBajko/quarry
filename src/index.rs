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
type Sides = (bool, bool, BTreeSet<String>);
type Declaration = (String, String, String, String, bool, String);

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
  consumes INTEGER NOT NULL DEFAULT 0
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
  PRIMARY KEY (from_repo, to_repo, kind, name)
);
CREATE INDEX edges_to ON edges (to_repo);
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
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct RebuildReport {
    pub(crate) repos: u32,
    pub(crate) pages: u32,
    pub(crate) edges: u32,
    pub(crate) warnings: Vec<String>,
    pub(crate) rebuilt: bool,
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
        for dir in docsrepo::read_repo_dirs(clone)? {
            let name = dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            names.insert(name.clone());
            let mut scan = RepoScan::default();
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
                    let (from, to) = if edge.produces {
                        (name.clone(), edge.other.clone())
                    } else {
                        (edge.other.clone(), name.clone())
                    };
                    if edge.produces {
                        scan.produces += 1;
                    } else {
                        scan.consumes += 1;
                    }
                    declarations.push((
                        from,
                        to,
                        edge.kind,
                        edge.name,
                        edge.produces,
                        format!("{name}/{relative}"),
                    ));
                }
            }
            transaction.execute(
                "INSERT OR REPLACE INTO repos (name, commit_sha, origin, newest_generated_date, pages, produces, consumes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![name, scan.commit, scan.origin, scan.newest_generated_date, scan.pages, scan.produces, scan.consumes],
            )?;
            report.repos += 1;
        }
        let mut merged: BTreeMap<EdgeKey, Sides> = BTreeMap::new();
        for (from, to, kind, name, produces, via) in declarations {
            let entry =
                merged
                    .entry((from, to, kind, name))
                    .or_insert((false, false, BTreeSet::new()));
            if produces {
                entry.0 = true;
            } else {
                entry.1 = true;
            }
            entry.2.insert(via);
        }
        for ((from, to, kind, name), (by_producer, by_consumer, via)) in merged {
            let declared_by = match (by_producer, by_consumer) {
                (true, true) => "both",
                (true, false) => "producer",
                _ => "consumer",
            };
            let missing = !names.contains(&from) || !names.contains(&to);
            let key = format!("{kind} {name}");
            let site_unverified = unverified.contains(&(from.clone(), key.clone()))
                || unverified.contains(&(to.clone(), key));
            let via_json = serde_json::to_string(&via.iter().collect::<Vec<_>>())?;
            transaction.execute(
                "INSERT OR REPLACE INTO edges (from_repo, to_repo, kind, name, declared_by, via, missing, site_unverified) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![from, to, kind, name, declared_by, via_json, missing as i64, site_unverified as i64],
            )?;
            report.edges += 1;
        }
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

impl Index {
    pub(crate) fn repos(&self) -> Result<Vec<RepoRow>> {
        let mut statement = self.connection.prepare(
            "SELECT name, commit_sha, origin, newest_generated_date, pages, produces, consumes FROM repos ORDER BY name",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(RepoRow {
                name: row.get(0)?,
                commit: row.get(1)?,
                origin: row.get(2)?,
                newest_generated_date: row.get(3)?,
                pages: row.get(4)?,
                produces: row.get(5)?,
                consumes: row.get(6)?,
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
            "SELECT from_repo, to_repo, kind, name, declared_by, via, missing, site_unverified FROM edges WHERE from_repo = ?1 ORDER BY to_repo, kind, name"
        } else {
            "SELECT from_repo, to_repo, kind, name, declared_by, via, missing, site_unverified FROM edges WHERE to_repo = ?1 ORDER BY from_repo, kind, name"
        };
        let mut statement = self.connection.prepare(sql)?;
        let rows = statement.query_map([repo], row_to_edge)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub(crate) fn edges_touching(&self, repo: &str) -> Result<Vec<Edge>> {
        let mut statement = self.connection.prepare(
            "SELECT from_repo, to_repo, kind, name, declared_by, via, missing, site_unverified FROM edges WHERE from_repo = ?1 OR to_repo = ?1 ORDER BY from_repo, to_repo, kind, name",
        )?;
        let rows = statement.query_map([repo], row_to_edge)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub(crate) fn repo_edges(&self, repo: &str) -> Result<(Vec<Edge>, Vec<Edge>)> {
        Ok((self.edges(repo, true)?, self.edges(repo, false)?))
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
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use rusqlite::Connection;

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
