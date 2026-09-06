//! One envelope, one exit-code contract, and the human layouts.

use serde::Serialize;
use serde_json::{Value, json};

use crate::check::CheckOut;
use crate::errors::QuarryError;
use crate::index::{Edge, RebuildReport, RepoRow};
use crate::query::{DepsResult, PathResult, RepoShow, SearchHit, SectionHit};

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct Meta {
    pub(crate) built_at_commit: Option<String>,
    pub(crate) synced_at: Option<String>,
    // A read command refreshes the clone first and answers either way, so a
    // failed refresh is reported here rather than raised.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) notes: Vec<String>,
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
    pub(crate) targets: Vec<crate::config::Target>,
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
    WriteMany(Vec<WriteOut>),
    Sync(SyncOut),
    Remove(RemoveOut),
    RemoveMany(Vec<RemoveOut>),
    Repos(Vec<RepoRow>),
    Files(FilesOut),
    Show(Box<RepoShow>),
    Section(Box<SectionHit>),
    Search(Vec<SearchHit>),
    Deps(Box<DepsResult>),
    Path(Box<PathResult>),
    Index(RebuildReport),
    Check(CheckOut),
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

    // Only this payload's content decides the exit code.
    pub(crate) fn exit_code(&self) -> u8 {
        match &self.payload {
            Payload::Check(out) if !out.breaks.is_empty() => 1,
            _ => 0,
        }
    }
}

pub(crate) fn count_breaks(n: usize) -> String {
    match n {
        0 => "no breaks".to_string(),
        1 => "1 break".to_string(),
        n => format!("{n} breaks"),
    }
}

pub(crate) fn render(response: &Response, json: bool, stale_note: Option<String>) -> String {
    if json {
        let value = json!({
            "ok": true,
            "built_at_commit": response.meta.built_at_commit,
            "synced_at": response.meta.synced_at,
            "notes": response.meta.notes,
            "result": payload_value(&response.payload),
        });
        return format!("{value}\n");
    }
    let mut text = String::new();
    for note in &response.meta.notes {
        text.push_str(note);
        text.push('\n');
    }
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
        Payload::WriteMany(outs) => serde_json::to_value(outs).unwrap_or(Value::Null),
        Payload::Sync(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Remove(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::RemoveMany(outs) => serde_json::to_value(outs).unwrap_or(Value::Null),
        Payload::Repos(rows) => serde_json::to_value(rows).unwrap_or(Value::Null),
        Payload::Files(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Show(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Section(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Search(hits) => serde_json::to_value(hits).unwrap_or(Value::Null),
        Payload::Deps(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Path(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Index(out) => serde_json::to_value(out).unwrap_or(Value::Null),
        Payload::Check(out) => serde_json::to_value(out).unwrap_or(Value::Null),
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
            for target in &out.targets {
                text.push_str(&format!("target {}: {}\n", target.name, target.docs_dir));
            }
            if let Some(repo) = &out.repo {
                text.push_str(&format!(
                    "this repo is `{repo}` (from origin, default branch {}). Next: quarry add\n",
                    out.default_branch
                ));
            }
            text
        }
        Payload::Write(out) => write_block(out, true),
        Payload::WriteMany(outs) => {
            let mut text: String = outs.iter().map(|out| write_block(out, false)).collect();
            if outs.iter().any(|out| out.result == "imported") {
                text.push_str("root index regenerated; pushed\n");
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
        Payload::Remove(out) => remove_block(out),
        Payload::RemoveMany(outs) => outs.iter().map(remove_block).collect(),
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
            if !out.repo.known_as.is_empty() {
                text.push_str(&format!("known as: {}\n", out.repo.known_as.join(", ")));
            }
            if out.produces.is_empty() && out.publications.is_empty() {
                text.push_str("produces: none\n");
            } else {
                for edge in &out.produces {
                    text.push_str(&format!(
                        "produces: {} {} -> {}{}\n",
                        edge.kind,
                        edge.name,
                        edge.to_repo,
                        edge_marks(edge, out.observed_file)
                    ));
                }
                for publication in &out.publications {
                    text.push_str(&format!(
                        "produces: {} {} -> (unknown)\n",
                        publication.kind, publication.name
                    ));
                }
            }
            if out.consumes.is_empty() {
                text.push_str("consumes: none\n");
            } else {
                for edge in &out.consumes {
                    text.push_str(&format!(
                        "consumes: {} {} <- {}{}\n",
                        edge.kind,
                        edge.name,
                        edge.from_repo,
                        edge_marks(edge, out.observed_file)
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
                match edge.declared_by.as_str() {
                    "both" | "observed" | "joined" => {}
                    "by-name" => marks.push_str(" (by name only)"),
                    side => marks.push_str(&format!(" (declared by {side} only)")),
                }
                marks.push_str(resolved_by(edge.resolved_by.as_deref()));
                marks.push_str(&declared_as(edge.as_declared.as_deref()));
                if edge.site_unverified {
                    marks.push_str(" (site unverified)");
                }
                marks.push_str(observed_mark(
                    &edge.declared_by,
                    edge.observed,
                    edge.missing,
                    out.observed_file,
                ));
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
            if let Some(observed) = &report.observed {
                text.push_str(&format!("observed edges: {}", observed.rows));
                if let Some(date) = &observed.generated_at {
                    text.push_str(&format!(" (generated {date})"));
                }
                text.push('\n');
            }
            for warning in &report.warnings {
                text.push_str(&format!("warning: {warning}\n"));
            }
            for entry in &report.unresolved {
                text.push_str(&format!(
                    "unresolved: {} ({})",
                    entry.declared,
                    entry.via.join(", ")
                ));
                if !entry.nearest.is_empty() {
                    text.push_str(&format!("; nearest: {}", entry.nearest.join(", ")));
                }
                text.push('\n');
            }
            for entry in &report.ambiguous {
                let (side, partners) = if entry.direction == "produces" {
                    ("produced by", "consumers")
                } else {
                    ("consumed by", "producers")
                };
                text.push_str(&format!(
                    "unresolved: {} {} {side} {}: {} {partners}, {}\n",
                    entry.kind,
                    entry.name,
                    entry.repo,
                    entry.candidates.len(),
                    entry.candidates.join(", ")
                ));
            }
            text
        }
        Payload::Check(out) => {
            let mut text = String::new();
            for contract in &out.contracts {
                let source = match &contract.model {
                    Some(model) => format!(
                        " (fields from {} § {model})",
                        crate::frontmatter::MODELS_PAGE
                    ),
                    None => String::new(),
                };
                text.push_str(&format!(
                    "{} produces {} {}{source}\n",
                    contract.target.as_deref().unwrap_or(&out.repo),
                    contract.kind,
                    contract.name
                ));
                for consumer in &contract.consumers {
                    let fields = if consumer.fields.is_empty() {
                        "no recorded fields".to_string()
                    } else {
                        consumer.fields.join(", ")
                    };
                    text.push_str(&format!(
                        "  {} reads {}   ({})\n",
                        consumer.repo,
                        fields,
                        consumer.generated_date.as_deref().unwrap_or("-")
                    ));
                    for line in &consumer.breaks {
                        text.push_str(&format!("  break: {line}\n"));
                    }
                    for line in &consumer.warnings {
                        text.push_str(&format!("  warning: {line}\n"));
                    }
                }
            }
            for warning in &out.warnings {
                text.push_str(&format!("warning: {warning}\n"));
            }
            for note in &out.notes {
                text.push_str(&format!("note: {note}\n"));
            }
            text.push_str(&count_breaks(out.breaks.len()));
            text.push('\n');
            text
        }
    }
}

// `trailer` is the closing line one run prints once, however many folders it
// wrote.
fn write_block(out: &WriteOut, trailer: bool) -> String {
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
            if trailer {
                text.push_str("root index regenerated; pushed\n");
            }
        }
    }
    text
}

fn remove_block(out: &RemoveOut) -> String {
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

// `(declared, never observed)` needs the file to be there to mean anything, and
// says nothing about an edge no traffic could ever back: one pointing at a repo
// that is not in the quarry, or a by-name lead that is not an edge at all.
fn observed_mark(
    declared_by: &str,
    observed: bool,
    missing: bool,
    file_present: bool,
) -> &'static str {
    if declared_by == "observed" {
        return " (observed, undeclared)";
    }
    if file_present && !observed && !missing && declared_by != "by-name" {
        return " (declared, never observed)";
    }
    ""
}

fn edge_marks(edge: &Edge, file_present: bool) -> String {
    let mut marks = resolved_by(edge.resolved_by.as_deref()).to_string();
    marks.push_str(&declared_as(edge.as_declared.as_deref()));
    if edge.site_unverified {
        marks.push_str(" (site unverified)");
    }
    marks.push_str(observed_mark(
        &edge.declared_by,
        edge.observed,
        edge.missing,
        file_present,
    ));
    marks
}

fn resolved_by(resolved_by: Option<&str>) -> &'static str {
    match resolved_by {
        Some("name") => " (resolved by name)",
        _ => "",
    }
}

fn declared_as(as_declared: Option<&str>) -> String {
    match as_declared {
        Some(declared) => format!(" (declared as {declared})"),
        None => String::new(),
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
    fn s15_index_report_lists_unresolved_with_nearest() {
        let report = RebuildReport {
            rebuilt: true,
            unresolved: vec![
                crate::index::Unresolved {
                    declared: "records-svc".to_string(),
                    via: vec!["ingest-api/09-interfaces.md".to_string()],
                    nearest: vec!["record-store".to_string(), "records-api".to_string()],
                },
                crate::index::Unresolved {
                    declared: "ghost".to_string(),
                    via: vec!["ingest-api/09-interfaces.md".to_string()],
                    nearest: Vec::new(),
                },
            ],
            ..RebuildReport::default()
        };
        let text = render(&Response::bare(Payload::Index(report)), false, None);
        assert!(
            text.contains(
                "unresolved: records-svc (ingest-api/09-interfaces.md); nearest: record-store, records-api\n"
            ),
            "{text}"
        );
        assert!(
            text.contains("unresolved: ghost (ingest-api/09-interfaces.md)\n"),
            "{text}"
        );
        assert!(
            !text.contains("ghost (ingest-api/09-interfaces.md);"),
            "{text}"
        );
    }

    #[test]
    fn s16_observed_marks_depend_on_the_file_being_present() {
        assert_eq!(
            observed_mark("observed", true, false, true),
            " (observed, undeclared)"
        );
        assert_eq!(
            observed_mark("both", false, false, true),
            " (declared, never observed)"
        );
        assert_eq!(observed_mark("both", false, false, false), "");
        assert_eq!(observed_mark("both", true, false, true), "");
        assert_eq!(observed_mark("producer", false, true, true), "");
        assert_eq!(observed_mark("by-name", false, false, true), "");
    }

    fn check_out(breaks: Vec<crate::check::ContractBreak>) -> CheckOut {
        CheckOut {
            repo: "record-store".to_string(),
            contracts: vec![crate::check::ContractOut {
                kind: "http".to_string(),
                name: "GET /records".to_string(),
                model: None,
                consumers: vec![crate::check::ConsumerOut {
                    repo: "report-builder".to_string(),
                    fields: vec![
                        "id".to_string(),
                        "created_at".to_string(),
                        "content_type".to_string(),
                    ],
                    generated_date: Some("2026-09-01".to_string()),
                    breaks: breaks
                        .iter()
                        .map(|b| format!("{} {}", b.field, b.reason))
                        .collect(),
                    warnings: Vec::new(),
                }],
                target: None,
            }],
            breaks,
            warnings: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn one_break() -> Vec<crate::check::ContractBreak> {
        vec![crate::check::ContractBreak {
            consumer: "report-builder".to_string(),
            kind: "http".to_string(),
            name: "GET /records".to_string(),
            field: "content_type".to_string(),
            reason: "no longer produced".to_string(),
            target: None,
        }]
    }

    #[test]
    fn s14_a_check_with_breaks_exits_one() {
        assert_eq!(
            Response::bare(Payload::Check(check_out(one_break()))).exit_code(),
            1
        );
        assert_eq!(
            Response::bare(Payload::Check(check_out(Vec::new()))).exit_code(),
            0
        );
        assert_eq!(Response::bare(Payload::Help(String::new())).exit_code(), 0);
    }

    #[test]
    fn s14_check_human_layout_matches_the_contract() {
        let text = render(
            &Response::bare(Payload::Check(check_out(one_break()))),
            false,
            None,
        );
        assert_eq!(
            text,
            "record-store produces http GET /records\n  report-builder reads id, created_at, content_type   (2026-09-01)\n  break: content_type no longer produced\n1 break\n"
        );
    }

    #[test]
    fn s14_the_break_count_is_pluralised_once() {
        assert_eq!(count_breaks(0), "no breaks");
        assert_eq!(count_breaks(1), "1 break");
        assert_eq!(count_breaks(3), "3 breaks");
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
