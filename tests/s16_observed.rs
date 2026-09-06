//! S16: observed edges.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use serde_json::Value;

const DATE: &str = "2026-09-05";

fn json(out: &std::process::Output) -> Value {
    serde_json::from_str(&stdout(out))
        .unwrap_or_else(|e| panic!("not one JSON document: {e}\n{}", stdout(out)))
}

/// `wired()` plus one observed-edges.json at the docs repo root, synced in.
fn with_observed(rows: &[(&str, &str, &str, &str, &str)]) -> Wired {
    with_observed_body(&observed_file(DATE, rows))
}

fn with_observed_body(body: &str) -> Wired {
    let wired = wired();
    wired.w.push_docs_root_file("observed-edges.json", body);
    let out = wired.w.run(&["sync"]);
    assert!(out.status.success(), "{}", stderr(&out));
    wired
}

fn edge_of<'a>(value: &'a Value, name: &str) -> &'a Value {
    value["result"]["edges"]
        .as_array()
        .unwrap_or_else(|| panic!("no edges array in {value}"))
        .iter()
        .find(|e| e["name"] == Value::String(name.to_string()))
        .unwrap_or_else(|| panic!("no edge {name} in {value}"))
}

#[test]
fn s16_traffic_no_page_declares_is_marked_undeclared() {
    let wired = with_observed(&[(
        "ingest-api",
        "report-builder",
        "HTTP",
        "GET /health",
        "2026-09-05",
    )]);
    let out = wired.w.run(&["docs", "deps", "ingest-api", "--downstream"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("http GET /health -> report-builder (observed, undeclared)\n"),
        "{text}"
    );
    assert!(
        text.contains("sqs file-ingest -> record-store (declared, never observed)\n"),
        "{text}"
    );

    let out = wired
        .w
        .run(&["--json", "docs", "deps", "ingest-api", "--downstream"]);
    let value = json(&out);
    assert_eq!(value["result"]["observed_file"], Value::Bool(true));
    let edge = edge_of(&value, "GET /health");
    assert_eq!(edge["declared_by"], Value::String("observed".to_string()));
    assert_eq!(edge["observed"], Value::Bool(true));
    assert_eq!(edge["last_seen"], Value::String(DATE.to_string()));
    assert_eq!(edge["missing"], Value::Bool(false));
}

#[test]
fn s16_a_declared_edge_seen_in_traffic_carries_no_mark() {
    let wired = with_observed(&[(
        "ingest-api",
        "record-store",
        "sqs",
        "file-ingest",
        "2026-09-04",
    )]);
    let out = wired.w.run(&["docs", "deps", "ingest-api", "--downstream"]);
    let text = stdout(&out);
    assert!(
        text.contains("  sqs file-ingest -> record-store\n"),
        "{text}"
    );

    let out = wired
        .w
        .run(&["--json", "docs", "deps", "ingest-api", "--downstream"]);
    let value = json(&out);
    let edge = edge_of(&value, "file-ingest");
    assert_eq!(edge["declared_by"], Value::String("both".to_string()));
    assert_eq!(edge["observed"], Value::Bool(true));
    assert_eq!(edge["last_seen"], Value::String("2026-09-04".to_string()));
    assert_eq!(edge["via"].as_array().map(Vec::len), Some(2), "{value}");
}

#[test]
fn s16_without_the_file_nothing_changes() {
    let wired = wired();
    let out = wired.w.run(&["docs", "deps", "ingest-api", "--downstream"]);
    let text = stdout(&out);
    assert!(!text.contains("observed"), "{text}");

    let out = wired
        .w
        .run(&["--json", "docs", "deps", "ingest-api", "--downstream"]);
    let value = json(&out);
    assert_eq!(value["result"]["observed_file"], Value::Bool(false));
    let edge = edge_of(&value, "file-ingest");
    assert_eq!(edge["observed"], Value::Bool(false));
    assert_eq!(edge["last_seen"], Value::Null);

    let out = wired.w.run(&["docs", "index"]);
    assert!(
        !stdout(&out).contains("observed edges:"),
        "{}",
        stdout(&out)
    );
    let out = wired.w.run(&["--json", "docs", "index", "--force"]);
    assert_eq!(json(&out)["result"]["observed"], Value::Null);
}

#[test]
fn s16_names_resolve_through_aliases_and_case() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page("2026-09-04")),
        (
            "09-interfaces.md",
            &produces_page("2026-09-04", "record-store", "sqs", "file-ingest"),
        ),
    ]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());

    let data = w.other_repo("record-store");
    w.write_docs_in(
        &data,
        &[
            ("00-index.md", &index_page("2026-09-03")),
            (
                "09-interfaces.md",
                &aliased_consumes_page(
                    "2026-09-03",
                    &["records-svc"],
                    "ingest-api",
                    "sqs",
                    "file-ingest",
                ),
            ),
        ],
    );
    w.commit_push_in(&data, "docs");
    assert!(w.run_in(&data, &["add"]).status.success());

    w.push_docs_root_file(
        "observed-edges.json",
        &observed_file(
            DATE,
            &[
                ("Ingest-API", "records-svc", "sqs", "file-ingest", DATE),
                ("ingest-api", "ghost-svc", "http", "GET /x", DATE),
                ("ingest-api", "record-storage", "http", "GET /y", DATE),
            ],
        ),
    );
    assert!(w.run(&["sync"]).status.success());

    let out = w.run(&["docs", "index", "--force"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("observed edges: 1 (generated 2026-09-05)\n"),
        "{text}"
    );
    assert!(
        text.contains(
            "warning: observed-edges.json: ingest-api -> ghost-svc http GET /x: ghost-svc is not in the quarry; row skipped\n"
        ),
        "{text}"
    );
    // A name that shares a prefix with a registered name or alias carries the
    // nearest-name hint; ghost-svc above shares none and carries no hint.
    assert!(
        text.contains(
            "warning: observed-edges.json: ingest-api -> record-storage http GET /y: record-storage is not in the quarry; row skipped; nearest: record-store, records-svc\n"
        ),
        "{text}"
    );

    let out = w.run(&["--json", "docs", "deps", "ingest-api", "--downstream"]);
    let value = json(&out);
    let edges = value["result"]["edges"].as_array().expect("edges");
    assert_eq!(edges.len(), 1, "{value}");
    assert_eq!(edges[0]["repo"], Value::String("record-store".to_string()));
    assert_eq!(edges[0]["observed"], Value::Bool(true));
    assert!(!stdout(&out).contains("ghost-svc"), "{}", stdout(&out));
}

#[test]
fn s16_docs_index_reports_the_file_on_a_current_index_too() {
    let wired = with_observed(&[
        ("ingest-api", "record-store", "sqs", "file-ingest", DATE),
        (
            "record-store",
            "report-builder",
            "http",
            "GET /records",
            DATE,
        ),
    ]);
    let out = wired.w.run(&["docs", "index"]);
    let text = stdout(&out);
    assert!(text.contains("index current\n"), "{text}");
    assert!(
        text.contains("observed edges: 2 (generated 2026-09-05)\n"),
        "{text}"
    );

    let out = wired.w.run(&["docs", "index", "--force"]);
    let text = stdout(&out);
    assert!(text.contains("index rebuilt"), "{text}");
    assert!(
        text.contains("observed edges: 2 (generated 2026-09-05)\n"),
        "{text}"
    );

    let out = wired.w.run(&["--json", "docs", "index"]);
    let value = json(&out);
    assert_eq!(value["result"]["observed"]["rows"], Value::from(2));
    assert_eq!(
        value["result"]["observed"]["generated_at"],
        Value::String(DATE.to_string())
    );
}

#[test]
fn s16_a_file_without_generated_at_prints_the_count_alone() {
    let wired = with_observed_body(&observed_file(
        "",
        &[("ingest-api", "report-builder", "http", "GET /health", "")],
    ));
    let out = wired.w.run(&["docs", "index", "--force"]);
    let text = stdout(&out);
    assert!(text.contains("observed edges: 1\n"), "{text}");
    assert!(!text.contains("generated"), "{text}");

    let out = wired
        .w
        .run(&["--json", "docs", "deps", "ingest-api", "--downstream"]);
    let value = json(&out);
    let edge = edge_of(&value, "GET /health");
    assert_eq!(edge["observed"], Value::Bool(true));
    assert_eq!(edge["last_seen"], Value::Null);
}

#[test]
fn s16_a_malformed_file_is_one_warning_not_a_failure() {
    let wired = wired();
    wired
        .w
        .push_docs_root_file("observed-edges.json", "{not json");
    let out = wired.w.run(&["sync"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));

    let out = wired.w.run(&["docs", "index", "--force"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    let warnings: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("warning: "))
        .collect();
    assert_eq!(warnings.len(), 1, "{text}");
    assert!(
        warnings[0].starts_with("warning: observed-edges.json: not valid JSON"),
        "{text}"
    );
    assert!(text.contains("observed edges: 0\n"), "{text}");

    let out = wired.w.run(&["docs", "deps", "ingest-api", "--downstream"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("sqs file-ingest -> record-store (declared, never observed)\n"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn s16_show_marks_both_directions() {
    let wired = with_observed(&[(
        "record-store",
        "report-builder",
        "http",
        "GET /records",
        DATE,
    )]);
    let out = wired.w.run(&["docs", "show", "record-store"]);
    let text = stdout(&out);
    assert!(
        text.contains("consumes: sqs file-ingest <- ingest-api (declared, never observed)\n"),
        "{text}"
    );
    assert!(
        text.contains("produces: http GET /records -> report-builder\n"),
        "{text}"
    );

    let out = wired.w.run(&["--json", "docs", "show", "record-store"]);
    let value = json(&out);
    assert_eq!(value["result"]["observed_file"], Value::Bool(true));
    assert_eq!(
        value["result"]["produces"][0]["observed"],
        Value::Bool(true)
    );
    assert_eq!(
        value["result"]["produces"][0]["last_seen"],
        Value::String(DATE.to_string())
    );
    assert_eq!(
        value["result"]["consumes"][0]["observed"],
        Value::Bool(false)
    );
    assert_eq!(value["result"]["consumes"][0]["last_seen"], Value::Null);
}

#[test]
fn s16_remove_lists_declared_not_observed_dangling() {
    let wired = with_observed(&[("ingest-api", "report-builder", "http", "GET /health", DATE)]);
    let out = wired.w.run_in(&wired.report_builder, &["remove"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("1 repos still declare edges to report-builder: record-store\n"),
        "{text}"
    );
    assert!(!text.contains("ingest-api"), "{text}");
}

#[test]
fn s16_observed_only_edges_are_walked_and_pathed() {
    let wired = with_observed(&[(
        "report-builder",
        "ingest-api",
        "http",
        "POST /reports",
        DATE,
    )]);
    let out = wired
        .w
        .run(&["docs", "deps", "ingest-api", "--downstream", "--depth", "0"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("http POST /reports -> ingest-api (cycle) (observed, undeclared)\n"),
        "{text}"
    );

    let out = wired
        .w
        .run(&["docs", "path", "report-builder", "ingest-api"]);
    let text = stdout(&out);
    assert!(
        text.contains("-[http POST /reports]-> ingest-api"),
        "{text}"
    );
    assert!(text.contains("1 hops\n"), "{text}");
}

#[test]
fn s16_a_route_seen_twice_keeps_the_latest_date() {
    let wired = with_observed(&[
        (
            "ingest-api",
            "report-builder",
            "http",
            "GET /health",
            "2026-09-04",
        ),
        (
            "ingest-api",
            "report-builder",
            "http",
            "GET /health",
            "2026-09-05",
        ),
    ]);
    let out = wired.w.run(&["docs", "index", "--force"]);
    assert!(
        stdout(&out).contains("observed edges: 2"),
        "{}",
        stdout(&out)
    );

    let out = wired
        .w
        .run(&["--json", "docs", "deps", "ingest-api", "--downstream"]);
    let value = json(&out);
    let edge = edge_of(&value, "GET /health");
    assert_eq!(edge["last_seen"], Value::String("2026-09-05".to_string()));
}
