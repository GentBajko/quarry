//! S15: aliases and unknown consumers.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use serde_json::Value;

const DATE: &str = "2026-09-04";

fn json(out: &std::process::Output) -> Value {
    serde_json::from_str(&stdout(out))
        .unwrap_or_else(|e| panic!("not one JSON document: {e}\n{}", stdout(out)))
}

/// ingest-api produces `sqs file-ingest` to whatever `to` says.
fn producer(to: &str) -> World {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page(DATE)),
        (
            "09-interfaces.md",
            &produces_page(DATE, to, "sqs", "file-ingest"),
        ),
    ]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    w
}

/// Registers a second repo with the pages given, then syncs ingest-api.
fn register(w: &World, name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let path = w.other_repo(name);
    w.write_docs_in(&path, files);
    w.commit_push_in(&path, "docs");
    let out = w.run_in(&path, &["add"]);
    assert!(out.status.success(), "{}", stderr(&out));
    path
}

/// ingest-api produces to `records-svc`; record-store answers to that alias.
fn aliased() -> World {
    let w = producer("records-svc");
    register(
        &w,
        "record-store",
        &[
            ("00-index.md", &index_page("2026-09-03")),
            (
                "09-interfaces.md",
                &aliased_consumes_page(
                    "2026-09-03",
                    &["records.internal", "records-svc"],
                    "ingest-api",
                    "sqs",
                    "file-ingest",
                ),
            ),
        ],
    );
    assert!(w.run(&["sync"]).status.success());
    w
}

/// ingest-api publishes `http GET /records` with no named consumer.
fn publisher() -> World {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page(DATE)),
        (
            "09-interfaces.md",
            &produces_unknown_page(DATE, "http", "GET /records"),
        ),
    ]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    w
}

#[test]
fn s15_an_alias_resolves_a_declared_name() {
    let w = aliased();
    let out = w.run(&["docs", "deps", "ingest-api", "--downstream"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("sqs file-ingest -> record-store (declared as records-svc)"),
        "{text}"
    );
    assert!(!text.contains("(not in quarry)"), "{text}");
    assert!(!text.contains("declared by"), "{text}");

    let value = json(&w.run(&["--json", "docs", "deps", "ingest-api", "--downstream"]));
    let edges = value["result"]["edges"].as_array().expect("edges");
    assert_eq!(edges.len(), 1, "{value}");
    assert_eq!(edges[0]["declared_by"], Value::from("both"));
    assert_eq!(edges[0]["as_declared"], Value::from("records-svc"));
    assert_eq!(edges[0]["missing"], Value::Bool(false));
    assert_eq!(edges[0]["by_name"], Value::Bool(false));
}

#[test]
fn s15_case_folds_before_aliases() {
    let w = producer("Record-Store");
    register(
        &w,
        "record-store",
        &[
            ("00-index.md", &index_page("2026-09-03")),
            (
                "09-interfaces.md",
                &consumes_page("2026-09-03", "ingest-api", "sqs", "file-ingest"),
            ),
        ],
    );
    assert!(w.run(&["sync"]).status.success());
    let text = stdout(&w.run(&["docs", "deps", "ingest-api", "--downstream"]));
    assert!(
        text.contains("sqs file-ingest -> record-store (declared as Record-Store)"),
        "{text}"
    );
}

#[test]
fn s15_an_exact_name_carries_no_declared_as() {
    let it = wired();
    let value = json(
        &it.w
            .run(&["--json", "docs", "deps", "ingest-api", "--downstream"]),
    );
    let edges = value["result"]["edges"].as_array().expect("edges");
    assert!(edges[0].get("as_declared").is_none(), "{value}");
    let text = stdout(&it.w.run(&["docs", "deps", "ingest-api", "--downstream"]));
    assert!(!text.contains("(declared as"), "{text}");
}

#[test]
fn s15_an_alias_claimed_twice_is_ignored_with_a_warning() {
    let w = producer("shared-name");
    for name in ["alpha", "beta"] {
        register(
            &w,
            name,
            &[
                ("00-index.md", &index_page("2026-09-03")),
                (
                    "09-interfaces.md",
                    &known_as_page("2026-09-03", &["shared-name"]),
                ),
            ],
        );
    }
    assert!(w.run(&["sync"]).status.success());
    let out = w.run(&["docs", "index", "--force"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("warning: alias shared-name claimed by alpha and beta; ignored"),
        "{text}"
    );
    assert!(
        text.contains("unresolved: shared-name (ingest-api/09-interfaces.md)"),
        "{text}"
    );
    let deps = stdout(&w.run(&["docs", "deps", "ingest-api", "--downstream"]));
    assert!(deps.contains("(not in quarry)"), "{deps}");
}

#[test]
fn s15_an_alias_equal_to_another_repos_name_is_ignored() {
    let it = wired();
    it.w.write_docs_in(
        &it.report_builder,
        &[(
            "09-interfaces.md",
            &aliased_consumes_page(
                "2026-09-02",
                &["record-store"],
                "record-store",
                "http",
                "GET /records",
            ),
        )],
    );
    it.w.commit_push_in(&it.report_builder, "claim an alias");
    assert!(
        it.w.run_in(&it.report_builder, &["update"])
            .status
            .success()
    );
    assert!(it.w.run(&["sync"]).status.success());

    let text = stdout(&it.w.run(&["docs", "index", "--force"]));
    assert!(
        text.contains(
            "warning: alias record-store claimed by report-builder is repo record-store's name; ignored"
        ),
        "{text}"
    );
    let deps = stdout(&it.w.run(&["docs", "deps", "ingest-api", "--downstream"]));
    assert!(deps.contains("-> record-store"), "{deps}");
    assert!(!deps.contains("(declared as"), "{deps}");
}

#[test]
fn s15_unresolved_targets_list_the_nearest_names() {
    let w = producer("records-svc");
    register(
        &w,
        "record-store",
        &[("00-index.md", &index_page("2026-09-03"))],
    );
    assert!(w.run(&["sync"]).status.success());
    let text = stdout(&w.run(&["docs", "index", "--force"]));
    assert!(
        text.contains(
            "unresolved: records-svc (ingest-api/09-interfaces.md); nearest: record-store"
        ),
        "{text}"
    );
    let value = json(&w.run(&["--json", "docs", "index", "--force"]));
    let unresolved = value["result"]["unresolved"]
        .as_array()
        .expect("unresolved");
    assert_eq!(unresolved.len(), 1, "{value}");
    assert_eq!(unresolved[0]["declared"], Value::from("records-svc"));
    assert_eq!(
        unresolved[0]["via"],
        Value::from(vec!["ingest-api/09-interfaces.md"])
    );
    assert_eq!(unresolved[0]["nearest"], Value::from(vec!["record-store"]));
}

#[test]
fn s15_known_as_appears_in_list_json_and_show() {
    let w = aliased();
    let value = json(&w.run(&["--json", "docs", "list"]));
    let rows = value["result"].as_array().expect("rows");
    let row = rows
        .iter()
        .find(|r| r["name"] == "record-store")
        .expect("record-store row");
    assert_eq!(
        row["known_as"],
        Value::from(vec!["records-svc", "records.internal"])
    );
    let producer_row = rows
        .iter()
        .find(|r| r["name"] == "ingest-api")
        .expect("ingest-api row");
    assert_eq!(producer_row["known_as"], Value::from(Vec::<String>::new()));

    let shown = stdout(&w.run(&["docs", "show", "record-store"]));
    assert!(
        shown.contains("known as: records-svc, records.internal"),
        "{shown}"
    );
    assert!(
        !stdout(&w.run(&["docs", "show", "ingest-api"])).contains("known as:"),
        "{shown}"
    );
}

#[test]
fn s15_show_marks_an_aliased_edge_as_declared() {
    let w = aliased();
    let shown = stdout(&w.run(&["docs", "show", "ingest-api"]));
    assert!(
        shown.contains("produces: sqs file-ingest -> record-store (declared as records-svc)"),
        "{shown}"
    );
    // The string sits on the edge, not on one end, so the consumer's own
    // `show` repeats it.
    let other = stdout(&w.run(&["docs", "show", "record-store"]));
    assert!(
        other.contains("consumes: sqs file-ingest <- ingest-api (declared as records-svc)"),
        "{other}"
    );
}

#[test]
fn s15_known_as_in_the_index_page_still_resolves() {
    let w = producer("records-svc");
    register(
        &w,
        "record-store",
        &[
            (
                "00-index.md",
                &known_as_page("2026-09-03", &["records-svc"]),
            ),
            (
                "09-interfaces.md",
                &consumes_page("2026-09-03", "ingest-api", "sqs", "file-ingest"),
            ),
        ],
    );
    assert!(w.run(&["sync"]).status.success());
    let text = stdout(&w.run(&["docs", "deps", "ingest-api", "--downstream"]));
    assert!(
        text.contains("sqs file-ingest -> record-store (declared as records-svc)"),
        "{text}"
    );
}

#[test]
fn s15_a_scalar_known_as_is_a_warning() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page(DATE)),
        (
            "09-interfaces.md",
            &format!("---\ngenerated_date: {DATE}\nknown_as: records-svc\n---\n\n## Consumes\n"),
        ),
    ]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    let out = w.run(&["docs", "index", "--force"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("warning: ingest-api/09-interfaces.md: known_as is not a list"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn s15_a_publication_to_unknown_is_not_an_edge() {
    let w = publisher();
    let shown = stdout(&w.run(&["docs", "show", "ingest-api"]));
    assert!(
        shown.contains("produces: http GET /records -> (unknown)"),
        "{shown}"
    );
    assert!(!shown.contains("produces: none"), "{shown}");

    let indexed = stdout(&w.run(&["docs", "index", "--force"]));
    assert!(indexed.contains("0 edges"), "{indexed}");
    assert!(!indexed.contains("warning:"), "{indexed}");

    let value = json(&w.run(&["--json", "docs", "list"]));
    let rows = value["result"].as_array().expect("rows");
    assert_eq!(rows[0]["produces"], Value::from(1));

    let deps = stdout(&w.run(&["docs", "deps", "ingest-api", "--downstream"]));
    assert!(deps.contains("no edges declared for ingest-api"), "{deps}");
}

#[test]
fn s15_unknown_is_case_insensitive() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page(DATE)),
        (
            "09-interfaces.md",
            &produces_page(DATE, "Unknown", "http", "GET /records"),
        ),
    ]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    let shown = stdout(&w.run(&["docs", "show", "ingest-api"]));
    assert!(
        shown.contains("produces: http GET /records -> (unknown)"),
        "{shown}"
    );
    let indexed = stdout(&w.run(&["docs", "index", "--force"]));
    assert!(!indexed.contains("unresolved:"), "{indexed}");
}

/// ingest-api publishes `GET /records`; report-builder mentions it and
/// produces something of its own to `lonely`, which mentions nothing.
fn with_leads() -> World {
    let w = publisher();
    register(
        &w,
        "report-builder",
        &[
            ("00-index.md", &index_page("2026-09-03")),
            (
                "09-interfaces.md",
                &produces_and_mentions_page(
                    "2026-09-03",
                    "lonely",
                    "sqs",
                    "reports",
                    "GET /records",
                ),
            ),
        ],
    );
    register(&w, "lonely", &[("00-index.md", &index_page("2026-09-02"))]);
    assert!(w.run(&["sync"]).status.success());
    w
}

#[test]
fn s15_deps_lists_possible_consumers_by_name() {
    let w = with_leads();
    let out = w.run(&["docs", "deps", "ingest-api", "--downstream", "--depth", "0"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("http GET /records -> report-builder (by name only)"),
        "{text}"
    );
    assert!(!text.contains("sqs reports"), "{text}");
    assert!(!text.contains("lonely"), "{text}");

    let value = json(&w.run(&[
        "--json",
        "docs",
        "deps",
        "ingest-api",
        "--downstream",
        "--depth",
        "0",
    ]));
    let edges = value["result"]["edges"].as_array().expect("edges");
    assert_eq!(edges.len(), 1, "{value}");
    assert_eq!(edges[0]["declared_by"], Value::from("by-name"));
    assert_eq!(edges[0]["by_name"], Value::Bool(true));
    assert_eq!(edges[0]["depth"], Value::from(1));
    assert_eq!(
        edges[0]["via"],
        Value::from(vec!["ingest-api/09-interfaces.md"])
    );
    assert_eq!(value["result"]["repos"], Value::from(0));
}

#[test]
fn s15_the_producer_is_never_its_own_lead() {
    let w = with_leads();
    let value = json(&w.run(&[
        "--json",
        "docs",
        "deps",
        "ingest-api",
        "--downstream",
        "--depth",
        "0",
    ]));
    let edges = value["result"]["edges"].as_array().expect("edges");
    assert!(edges.iter().all(|e| e["repo"] != "ingest-api"), "{value}");
}

#[test]
fn s15_a_mention_outside_the_interfaces_page_is_not_a_lead() {
    let w = publisher();
    register(
        &w,
        "report-builder",
        &[
            ("00-index.md", &index_page("2026-09-03")),
            (
                "01-architecture.md",
                "---\ngenerated_date: 2026-09-03\n---\n\n## Layers\n\nWe call GET /records hourly.\n",
            ),
        ],
    );
    assert!(w.run(&["sync"]).status.success());
    let text = stdout(&w.run(&["docs", "deps", "ingest-api", "--downstream", "--depth", "0"]));
    assert!(text.contains("no edges declared for ingest-api"), "{text}");
}

#[test]
fn s15_a_consumer_that_declares_the_edge_is_not_listed_twice() {
    let w = publisher();
    register(
        &w,
        "report-builder",
        &[
            ("00-index.md", &index_page("2026-09-03")),
            (
                "09-interfaces.md",
                &consumes_page("2026-09-03", "ingest-api", "http", "GET /records"),
            ),
        ],
    );
    assert!(w.run(&["sync"]).status.success());
    let text = stdout(&w.run(&["docs", "deps", "ingest-api", "--downstream", "--depth", "0"]));
    assert_eq!(text.matches("report-builder").count(), 1, "{text}");
    assert!(text.contains("(declared by consumer only)"), "{text}");
    assert!(!text.contains("(by name only)"), "{text}");
}

#[test]
fn s15_upstream_never_lists_by_name() {
    let w = with_leads();
    let text = stdout(&w.run(&[
        "docs",
        "deps",
        "report-builder",
        "--upstream",
        "--depth",
        "0",
    ]));
    assert!(
        text.contains("no edges declared for report-builder"),
        "{text}"
    );
}

#[test]
fn s15_a_publication_below_the_depth_limit_truncates() {
    let w = producer("record-store");
    register(
        &w,
        "record-store",
        &[
            ("00-index.md", &index_page("2026-09-03")),
            (
                "09-interfaces.md",
                &produces_unknown_page("2026-09-03", "http", "GET /records"),
            ),
        ],
    );
    assert!(w.run(&["sync"]).status.success());
    let text = stdout(&w.run(&["docs", "deps", "ingest-api", "--downstream", "--depth", "1"]));
    assert!(text.contains("truncated"), "{text}");
}

#[test]
fn s15_remove_lists_an_aliased_consumer_as_dangling() {
    let w = aliased();
    let out = w.run(&["remove"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("removed ingest-api"), "{text}");
    assert!(text.contains("still declare edges to ingest-api"), "{text}");
    assert!(text.contains("record-store"), "{text}");
}

#[test]
fn s15_the_repo_total_leaves_out_by_name_rows() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page(DATE)),
        (
            "09-interfaces.md",
            "---\ngenerated_date: 2026-09-04\nproduces:\n  - kind: sqs\n    name: file-ingest\n    to: record-store\n  - kind: http\n    name: GET /records\n    to: unknown\n---\n\n## Produces\n\n| Kind | Name |\n|---|---|\n| sqs | file-ingest |\n| http | GET /records |\n",
        ),
    ]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    register(&w, "record-store", &[("00-index.md", &index_page(DATE))]);
    register(
        &w,
        "report-builder",
        &[
            ("00-index.md", &index_page(DATE)),
            (
                "09-interfaces.md",
                "---\ngenerated_date: 2026-09-03\n---\n\n## Consumes\n\n### GET /records (v2)\n\n| Field | Type |\n|---|---|\n| id | string |\n",
            ),
        ],
    );
    assert!(w.run(&["sync"]).status.success());
    let text = stdout(&w.run(&["docs", "deps", "ingest-api", "--downstream", "--depth", "0"]));
    assert!(text.contains("sqs file-ingest -> record-store"), "{text}");
    assert!(
        text.contains("http GET /records -> report-builder (by name only)"),
        "{text}"
    );
    // Two repo names on screen, one of them a lead: the total counts the edge
    // only, which is what README's Cross-repo edges section promises.
    assert!(text.contains("1 repos, depth 1"), "{text}");
}
