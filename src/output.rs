//! One envelope, one exit-code contract, and the human layouts.

use serde::Serialize;
use serde_json::{Value, json};

use crate::errors::QuarryError;
use crate::index::{RebuildReport, RepoRow};
use crate::query::{DepsResult, PathResult, RepoShow, SearchHit, SectionHit};

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct Meta {
    pub(crate) built_at_commit: Option<String>,
    pub(crate) synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WriteOut {
    pub(crate) repo: String,
    pub(crate) from: Option<String>,
    pub(crate) to: String,
    pub(crate) files: usize,
    pub(crate) result: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct InitOut {
    pub(crate) repo: Option<String>,
    pub(crate) url: String,
    pub(crate) docs_dir: String,
    pub(crate) default_branch: String,
    pub(crate) clone: String,
    pub(crate) cloned: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SyncOut {
    pub(crate) pulled: bool,
    pub(crate) index: String,
    pub(crate) repos: u32,
    pub(crate) pages: u32,
    pub(crate) edges: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RemoveOut {
    pub(crate) repo: String,
    pub(crate) removed: bool,
    pub(crate) dangling: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FilesOut {
    pub(crate) repo: String,
    pub(crate) commit: Option<String>,
    pub(crate) files: Vec<FileRow>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FileRow {
    pub(crate) path: String,
    pub(crate) generated_date: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum Payload {
    Help(String),
    Init(InitOut),
    Write(WriteOut),
    Sync(SyncOut),
    Remove(RemoveOut),
    Repos(Vec<RepoRow>),
    Files(FilesOut),
    Show(Box<RepoShow>),
    Section(Box<SectionHit>),
    Search(Vec<SearchHit>),
    Deps(Box<DepsResult>),
    Path(Box<PathResult>),
    Index(RebuildReport),
}

#[derive(Debug, Clone)]
pub(crate) struct Response {
    pub(crate) meta: Meta,
    pub(crate) payload: Payload,
}

impl Response {
    pub(crate) fn bare(payload: Payload) -> Self {
        Self {
            meta: Meta::default(),
            payload,
        }
    }
}

pub(crate) fn render(response: &Response, json: bool, stale_note: Option<String>) -> String {
    if json {
        let value = json!({
            "ok": true,
            "built_at_commit": response.meta.built_at_commit,
            "synced_at": response.meta.synced_at,
            "result": payload_value(&response.payload),
        });
        return format!("{value}\n");
    }
    let mut text = String::new();
    if let Some(note) = stale_note {
        text.push_str(&note);
        text.push('\n');
    }
    text.push_str(&human(&response.payload));
    text
}

pub(crate) fn error_envelope(error: &QuarryError) -> String {
    let value = json!({
        "ok": false,
        "code": error.exit_code(),
        "error": error.to_string(),
        "result": Value::Null,
    });
    format!("{value}\n")
}

fn payload_value(payload: &Payload) -> Value {
    match payload {
        Payload::Help(text) => json!({ "help": text }),
        Payload::Init(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Write(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Sync(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Remove(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Repos(rows) => serde_json::to_value(rows).unwrap_or(Value::Null),
        Payload::Files(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Show(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Section(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Search(hits) => serde_json::to_value(hits).unwrap_or(Value::Null),
        Payload::Deps(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Path(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Index(out) => serde_json::to_value(out).unwrap_or(Value::Null),
    }
}

fn human(payload: &Payload) -> String {
    match payload {
        Payload::Help(text) => text.clone(),
        Payload::Init(out) => {
            let mut text = String::new();
            for note in &out.notes {
                text.push_str(note);
                text.push('\n');
            }
            if out.cloned {
                text.push_str(&format!("cloned {} -> {}\n", out.url, out.clone));
            } else {
                text.push_str(&format!("clone current at {}\n", out.clone));
            }
            text.push_str("wrote .quarry/.config and .quarry/.gitignore\n");
            if let Some(repo) = &out.repo {
                text.push_str(&format!(
                    "this repo is `{repo}` (from origin, default branch {}). Next: quarry add\n",
                    out.default_branch
                ));
            }
            text
        }
        Payload::Write(out) => {
            let mut text = String::new();
            for note in &out.notes {
                text.push_str(note);
                text.push('\n');
            }
            match out.result.as_str() {
                "current" => text.push_str(&format!("{} @ {}: current\n", out.repo, out.to)),
                "skipped" => text.push_str(&format!(
                    "{}: docs repo already holds a newer commit, skipped\n",
                    out.repo
                )),
                _ => {
                    text.push_str(&format!(
                        "{} @ {} -> {} files imported\n",
                        out.repo, out.to, out.files
                    ));
                    text.push_str("root index regenerated; pushed\n");
                }
            }
            text
        }
        Payload::Sync(out) => {
            let mut text = String::new();
            for note in &out.notes {
                text.push_str(note);
                text.push('\n');
            }
            text.push_str(if out.pulled {
                "docs repo: updated\n"
            } else {
                "docs repo: up to date\n"
            });
            if out.index == "rebuilt" {
                text.push_str(&format!(
                    "index: rebuilt ({} repos, {} pages, {} edges)\n",
                    out.repos, out.pages, out.edges
                ));
            } else {
                text.push_str(&format!("index: current ({} repos)\n", out.repos));
            }
            text
        }
        Payload::Remove(out) => {
            let mut text = String::new();
            if out.removed {
                text.push_str(&format!("removed {}\n", out.repo));
            } else {
                text.push_str(&format!("{} is not in the docs repo\n", out.repo));
            }
            if !out.dangling.is_empty() {
                text.push_str(&format!(
                    "{} repos still declare edges to {}: {}\n",
                    out.dangling.len(),
                    out.repo,
                    out.dangling.join(", ")
                ));
            }
            text
        }
        Payload::Repos(rows) => {
            if rows.is_empty() {
                return "no repos yet; run quarry add in a repo\n".to_string();
            }
            let mut text = format!(
                "{:<28} {:<12} {:>5} {:>9} {:>9}\n",
                "repo", "newest", "pages", "produces", "consumes"
            );
            for row in rows {
                text.push_str(&format!(
                    "{:<28} {:<12} {:>5} {:>9} {:>9}\n",
                    row.name,
                    row.newest_generated_date.as_deref().unwrap_or("-"),
                    row.pages,
                    row.produces,
                    row.consumes
                ));
            }
            text
        }
        Payload::Files(out) => {
            let mut text = format!(
                "{} @ {}\n",
                out.repo,
                out.commit
                    .as_deref()
                    .map(|c| c.chars().take(7).collect::<String>())
                    .unwrap_or_else(|| "-".to_string())
            );
            for file in &out.files {
                text.push_str(&format!(
                    "  {:<40} {}\n",
                    file.path,
                    file.generated_date.as_deref().unwrap_or("")
                ));
            }
            text
        }
        Payload::Show(out) => {
            let mut text = format!(
                "{} @ {} (newest doc {})\n",
                out.repo.name,
                out.repo
                    .commit
                    .as_deref()
                    .map(|c| c.chars().take(7).collect::<String>())
                    .unwrap_or_else(|| "-".to_string()),
                out.repo.newest_generated_date.as_deref().unwrap_or("-")
            );
            if out.produces.is_empty() {
                text.push_str("produces: none\n");
            } else {
                for edge in &out.produces {
                    text.push_str(&format!(
                        "produces: {} {} -> {}\n",
                        edge.kind, edge.name, edge.to_repo
                    ));
                }
            }
            if out.consumes.is_empty() {
                text.push_str("consumes: none\n");
            } else {
                for edge in &out.consumes {
                    text.push_str(&format!(
                        "consumes: {} {} <- {}\n",
                        edge.kind, edge.name, edge.from_repo
                    ));
                }
            }
            if let Some(overview) = &out.overview {
                text.push_str(&format!("\n# {}\n{}\n", overview.heading, overview.body));
            }
            text
        }
        Payload::Section(hit) => format!(
            "{}/{} § {}   {}\n\n{}\n",
            hit.repo,
            hit.file,
            hit.heading,
            hit.generated_date.as_deref().unwrap_or("-"),
            hit.body
        ),
        Payload::Search(hits) => {
            let mut text = String::new();
            for hit in hits {
                text.push_str(&format!(
                    "{:<20} {:<28} § {:<32} {}\n",
                    hit.repo,
                    hit.file,
                    hit.heading,
                    hit.generated_date.as_deref().unwrap_or("-")
                ));
            }
            text.push_str(&format!("{} hits\n", hits.len()));
            text
        }
        Payload::Deps(out) => {
            if out.edges.is_empty() {
                return format!(
                    "no edges declared for {}; try quarry docs search\n",
                    out.repo
                );
            }
            let mut text = format!("{}\n", out.repo);
            let arrow = if out.direction == "downstream" {
                "->"
            } else {
                "<-"
            };
            for edge in &out.edges {
                let mut marks = String::new();
                if edge.missing {
                    marks.push_str(" (not in quarry)");
                }
                if edge.cycle {
                    marks.push_str(" (cycle)");
                }
                if edge.declared_by != "both" {
                    marks.push_str(&format!(" (declared by {} only)", edge.declared_by));
                }
                text.push_str(&format!(
                    "{}{} {} {} {}{}\n",
                    "  ".repeat(edge.depth as usize),
                    edge.kind,
                    edge.name,
                    arrow,
                    edge.repo,
                    marks
                ));
            }
            text.push_str(&format!(
                "{} repos, depth {}{}\n",
                out.repos,
                out.max_depth,
                if out.truncated { ", truncated" } else { "" }
            ));
            text
        }
        Payload::Path(out) => match &out.path {
            None => format!("no path from {} to {}\n", out.from, out.to),
            Some(hops) if hops.is_empty() => "same repo\n".to_string(),
            Some(hops) => {
                let mut text = String::new();
                if out.direction == "reverse" {
                    text.push_str(&format!(
                        "reverse path ({} produces for {})\n",
                        out.to, out.from
                    ));
                }
                text.push_str(&hops[0].from);
                for hop in hops {
                    text.push_str(&format!(" -[{} {}]-> {}", hop.kind, hop.name, hop.to));
                }
                text.push('\n');
                text.push_str(&format!("{} hops\n", hops.len()));
                text
            }
        },
        Payload::Index(report) => {
            let mut text = if report.rebuilt {
                format!(
                    "index rebuilt: {} repos, {} pages, {} edges\n",
                    report.repos, report.pages, report.edges
                )
            } else {
                "index current\n".to_string()
            };
            for warning in &report.warnings {
                text.push_str(&format!("warning: {warning}\n"));
            }
            text
        }
    }
}

pub(crate) fn stale_note(synced_at: Option<&str>, now: jiff::Timestamp) -> Option<String> {
    let Some(synced) = synced_at else {
        return Some("docs clone never synced; run quarry sync".to_string());
    };
    let parsed: jiff::Timestamp = synced.parse().ok()?;
    let hours = now.duration_since(parsed).as_hours();
    if hours >= 24 {
        return Some(format!(
            "docs clone last synced {}h ago; run quarry sync",
            hours
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn s13_error_envelope_is_one_json_document() {
        let text = error_envelope(&QuarryError::refusal("unknown repo x"));
        let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        assert_eq!(value["ok"], json!(false));
        assert_eq!(value["code"], json!(1));
        assert_eq!(value["error"], json!("unknown repo x"));
    }

    #[test]
    fn s13_external_failure_is_code_two() {
        let text = error_envelope(&QuarryError::external("docs repo busy, retry"));
        let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        assert_eq!(value["code"], json!(2));
    }

    #[test]
    fn s5_stale_note_appears_after_a_day() {
        let now: jiff::Timestamp = "2026-09-05T12:00:00Z"
            .parse()
            .unwrap_or(jiff::Timestamp::UNIX_EPOCH);
        assert!(stale_note(Some("2026-09-05T06:00:00Z"), now).is_none());
        assert!(stale_note(Some("2026-09-03T06:00:00Z"), now).is_some());
        assert!(stale_note(None, now).is_some());
    }
}
