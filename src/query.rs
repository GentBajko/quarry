//! The read side, over the index only.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::Serialize;

use crate::errors::{QuarryError, Result};
use crate::index::{Edge, Index, PageRow, Publication, RepoRow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Downstream,
    Upstream,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SectionHit {
    pub(crate) repo: String,
    pub(crate) file: String,
    pub(crate) heading: String,
    pub(crate) generated_date: Option<String>,
    pub(crate) body: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SearchHit {
    pub(crate) repo: String,
    pub(crate) file: String,
    pub(crate) heading: String,
    pub(crate) generated_date: Option<String>,
    pub(crate) snippet: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RepoShow {
    pub(crate) repo: RepoRow,
    pub(crate) produces: Vec<Edge>,
    pub(crate) consumes: Vec<Edge>,
    pub(crate) overview: Option<SectionHit>,
    pub(crate) publications: Vec<Publication>,
    pub(crate) observed_file: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DepEdge {
    pub(crate) repo: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) depth: u32,
    pub(crate) declared_by: String,
    pub(crate) missing: bool,
    pub(crate) cycle: bool,
    pub(crate) via: Vec<String>,
    pub(crate) site_unverified: bool,
    pub(crate) by_name: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) as_declared: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) resolved_by: Option<String>,
    pub(crate) observed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) last_seen: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DepsResult {
    pub(crate) repo: String,
    pub(crate) direction: String,
    pub(crate) edges: Vec<DepEdge>,
    pub(crate) repos: usize,
    pub(crate) max_depth: u32,
    pub(crate) truncated: bool,
    pub(crate) observed_file: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PathHop {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) kind: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PathResult {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) path: Option<Vec<PathHop>>,
    pub(crate) direction: String,
}

pub(crate) fn repos(index: &Index) -> Result<Vec<RepoRow>> {
    index.repos()
}

pub(crate) fn files(index: &Index, repo: &str) -> Result<Vec<PageRow>> {
    require_repo(index, repo)?;
    index.pages(repo)
}

pub(crate) fn show(index: &Index, repo: &str) -> Result<RepoShow> {
    require_repo(index, repo)?;
    let row = index
        .repo(repo)?
        .ok_or_else(|| QuarryError::refusal(format!("unknown repo {repo}")))?;
    let (produces, consumes) = index.repo_edges(repo)?;
    let overview = first_section(index, repo, "00-index.md")?;
    let publications = index.publications(repo)?;
    Ok(RepoShow {
        repo: row,
        produces,
        consumes,
        overview,
        publications,
        observed_file: index.observed.is_some(),
    })
}

fn first_section(index: &Index, repo: &str, file: &str) -> Result<Option<SectionHit>> {
    let mut statement = index.connection.prepare(
        "SELECT s.heading, s.body, p.generated_date FROM sections s LEFT JOIN pages p ON p.repo = s.repo AND p.path = s.path WHERE s.repo = ?1 AND s.path = ?2 LIMIT 1",
    )?;
    let mut rows = statement.query(rusqlite::params![repo, file])?;
    match rows.next()? {
        Some(row) => Ok(Some(SectionHit {
            repo: repo.to_string(),
            file: file.to_string(),
            heading: row.get(0)?,
            body: row.get(1)?,
            generated_date: row.get(2)?,
        })),
        None => Ok(None),
    }
}

pub(crate) fn section(index: &Index, repo: &str, heading: &str) -> Result<SectionHit> {
    require_repo(index, repo)?;
    let normalized = crate::frontmatter::normalize_heading(heading);
    if normalized.is_empty() {
        return Err(QuarryError::refusal("empty heading"));
    }
    let exact = matching(index, repo, &normalized, false)?;
    let candidates = if exact.is_empty() {
        matching(index, repo, &normalized, true)?
    } else {
        exact
    };
    match candidates.len() {
        1 => Ok(candidates.into_iter().next().unwrap_or_else(|| SectionHit {
            repo: repo.to_string(),
            file: String::new(),
            heading: String::new(),
            generated_date: None,
            body: String::new(),
        })),
        0 => Err(QuarryError::refusal(format!(
            "no section \"{heading}\" in {repo}{}",
            nearest(index, repo, &normalized)?
        ))),
        _ => Err(QuarryError::refusal(format!(
            "\"{heading}\" matches {} sections in {repo}: {}",
            candidates.len(),
            candidates
                .iter()
                .map(|c| format!("{} § {}", c.file, c.heading))
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn matching(index: &Index, repo: &str, normalized: &str, prefix: bool) -> Result<Vec<SectionHit>> {
    let sql = if prefix {
        "SELECT s.path, s.heading, s.body, p.generated_date FROM sections s LEFT JOIN pages p ON p.repo = s.repo AND p.path = s.path WHERE s.repo = ?1 AND s.normalized LIKE ?2 || '%' ORDER BY s.path"
    } else {
        "SELECT s.path, s.heading, s.body, p.generated_date FROM sections s LEFT JOIN pages p ON p.repo = s.repo AND p.path = s.path WHERE s.repo = ?1 AND s.normalized = ?2 ORDER BY s.path"
    };
    let mut statement = index.connection.prepare(sql)?;
    let rows = statement.query_map(rusqlite::params![repo, normalized], |row| {
        Ok(SectionHit {
            repo: repo.to_string(),
            file: row.get(0)?,
            heading: row.get(1)?,
            body: row.get(2)?,
            generated_date: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
}

fn nearest(index: &Index, repo: &str, normalized: &str) -> Result<String> {
    let mut statement = index
        .connection
        .prepare("SELECT normalized, heading, path FROM sections WHERE repo = ?1")?;
    let rows = statement.query_map([repo], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut scored: Vec<(usize, String)> = Vec::new();
    for row in rows {
        let (candidate, heading, path) = row?;
        let shared = candidate
            .chars()
            .zip(normalized.chars())
            .take_while(|(a, b)| a == b)
            .count();
        if shared > 0 {
            scored.push((shared, format!("{path} § {heading}")));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.dedup_by(|a, b| a.1 == b.1);
    if scored.is_empty() {
        return Ok(String::new());
    }
    let nearest: Vec<String> = scored.into_iter().take(5).map(|(_, s)| s).collect();
    Ok(format!("; nearest: {}", nearest.join(", ")))
}

pub(crate) fn search(
    index: &Index,
    term: &str,
    repo: Option<&str>,
    limit: u32,
) -> Result<Vec<SearchHit>> {
    if let Some(repo) = repo {
        require_repo(index, repo)?;
    }
    let quoted = format!("\"{}\"", term.replace('"', "\"\""));
    let sql = "SELECT s.repo, s.path, s.heading, snippet(sections, 5, '', '', ' … ', 12), p.generated_date
               FROM sections s LEFT JOIN pages p ON p.repo = s.repo AND p.path = s.path
               WHERE sections MATCH ?1 AND (?2 IS NULL OR s.repo = ?2)
               ORDER BY bm25(sections), s.repo, s.path LIMIT ?3";
    let mut statement = index.connection.prepare(sql)?;
    let rows = statement.query_map(rusqlite::params![quoted, repo, limit], |row| {
        Ok(SearchHit {
            repo: row.get(0)?,
            file: row.get(1)?,
            heading: row.get(2)?,
            snippet: row.get::<_, String>(3)?.replace('\n', " "),
            generated_date: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
}

fn consumers_by_name(index: &Index, node: &str, name: &str) -> Result<Vec<String>> {
    let quoted = format!("\"{}\"", name.replace('"', "\"\""));
    let sql = "SELECT DISTINCT s.repo FROM sections s
               WHERE sections MATCH ?1 AND s.path = ?2 AND s.repo != ?3
               ORDER BY s.repo";
    let mut statement = index.connection.prepare(sql)?;
    let rows = statement.query_map(
        rusqlite::params![quoted, crate::frontmatter::INTERFACES_PAGE, node],
        |row| row.get::<_, String>(0),
    )?;
    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
}

// An edge closes a loop only when its far end sits on the path back to the
// start. A second contract to a repo already queued elsewhere in the walk is a
// parallel edge: it prints unmarked and is not queued again.
fn closes_a_loop(parents: &HashMap<String, String>, node: &str, other: &str) -> bool {
    let mut at = node;
    while let Some(up) = parents.get(at) {
        if up == other {
            return true;
        }
        at = up;
    }
    false
}

pub(crate) fn deps(
    index: &Index,
    repo: &str,
    direction: Direction,
    depth: u32,
) -> Result<DepsResult> {
    require_repo(index, repo)?;
    let downstream = direction == Direction::Downstream;
    let mut seen: HashSet<String> = HashSet::from([repo.to_string()]);
    let mut parents: HashMap<String, String> = HashMap::new();
    let mut frontier: VecDeque<(String, u32)> = VecDeque::from([(repo.to_string(), 0)]);
    let mut edges: Vec<DepEdge> = Vec::new();
    let mut truncated = false;
    let mut max_depth = 0;
    while let Some((node, at)) = frontier.pop_front() {
        if depth != 0 && at >= depth {
            if !index.edges(&node, downstream)?.is_empty()
                || (downstream && !index.publications(&node)?.is_empty())
            {
                truncated = true;
            }
            continue;
        }
        let node_edges = index.edges(&node, downstream)?;
        for edge in &node_edges {
            let other = if downstream {
                edge.to_repo.clone()
            } else {
                edge.from_repo.clone()
            };
            let cycle = other == node || closes_a_loop(&parents, &node, &other);
            edges.push(DepEdge {
                repo: other.clone(),
                kind: edge.kind.clone(),
                name: edge.name.clone(),
                depth: at + 1,
                declared_by: edge.declared_by.clone(),
                missing: edge.missing,
                cycle,
                via: edge.via.clone(),
                site_unverified: edge.site_unverified,
                by_name: false,
                as_declared: edge.as_declared.clone(),
                resolved_by: edge.resolved_by.clone(),
                observed: edge.observed,
                last_seen: edge.last_seen.clone(),
            });
            max_depth = max_depth.max(at + 1);
            if !seen.contains(&other) && !edge.missing {
                seen.insert(other.clone());
                parents.insert(other.clone(), node.clone());
                frontier.push_back((other, at + 1));
            }
        }
        if !downstream {
            continue;
        }
        // A by-name row is a lead: never expanded, never counted as a repo,
        // and skipped when the consumer already declared the edge.
        for publication in index.publications(&node)? {
            let wanted = crate::frontmatter::normalize_heading(&publication.name);
            for hit in consumers_by_name(index, &node, &publication.name)? {
                let declared = node_edges.iter().any(|e| {
                    e.to_repo == hit && crate::frontmatter::normalize_heading(&e.name) == wanted
                });
                if declared {
                    continue;
                }
                edges.push(DepEdge {
                    repo: hit,
                    kind: publication.kind.clone(),
                    name: publication.name.clone(),
                    depth: at + 1,
                    declared_by: "by-name".to_string(),
                    missing: false,
                    cycle: false,
                    via: publication.via.clone(),
                    site_unverified: false,
                    by_name: true,
                    as_declared: None,
                    resolved_by: None,
                    observed: false,
                    last_seen: None,
                });
                max_depth = max_depth.max(at + 1);
            }
        }
    }
    let repos = edges
        .iter()
        .filter(|e| !e.by_name)
        .map(|e| e.repo.clone())
        .collect::<HashSet<_>>()
        .len();
    Ok(DepsResult {
        repo: repo.to_string(),
        direction: if downstream {
            "downstream".to_string()
        } else {
            "upstream".to_string()
        },
        edges,
        repos,
        max_depth,
        truncated,
        observed_file: index.observed.is_some(),
    })
}

pub(crate) fn path(index: &Index, from: &str, to: &str) -> Result<PathResult> {
    require_repo(index, from)?;
    require_repo(index, to)?;
    if from == to {
        return Ok(PathResult {
            from: from.to_string(),
            to: to.to_string(),
            path: Some(Vec::new()),
            direction: "same".to_string(),
        });
    }
    if let Some(hops) = bfs(index, from, to)? {
        return Ok(PathResult {
            from: from.to_string(),
            to: to.to_string(),
            path: Some(hops),
            direction: "forward".to_string(),
        });
    }
    if let Some(hops) = bfs(index, to, from)? {
        return Ok(PathResult {
            from: from.to_string(),
            to: to.to_string(),
            path: Some(hops),
            direction: "reverse".to_string(),
        });
    }
    Ok(PathResult {
        from: from.to_string(),
        to: to.to_string(),
        path: None,
        direction: "none".to_string(),
    })
}

fn bfs(index: &Index, from: &str, to: &str) -> Result<Option<Vec<PathHop>>> {
    let mut seen: HashSet<String> = HashSet::from([from.to_string()]);
    let mut frontier: VecDeque<(String, Vec<PathHop>)> =
        VecDeque::from([(from.to_string(), Vec::new())]);
    while let Some((node, hops)) = frontier.pop_front() {
        for edge in index.edges(&node, true)? {
            let hop = PathHop {
                from: edge.from_repo.clone(),
                to: edge.to_repo.clone(),
                kind: edge.kind.clone(),
                name: edge.name.clone(),
            };
            let mut next = hops.clone();
            next.push(hop);
            if edge.to_repo == to {
                return Ok(Some(next));
            }
            if edge.missing || !seen.insert(edge.to_repo.clone()) {
                continue;
            }
            frontier.push_back((edge.to_repo.clone(), next));
        }
    }
    Ok(None)
}

fn require_repo(index: &Index, repo: &str) -> Result<()> {
    if index.has_repo(repo)? {
        return Ok(());
    }
    Err(QuarryError::refusal(format!("unknown repo {repo}")))
}
