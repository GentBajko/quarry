//! S7: graph traversal.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;

#[test]
fn s7_downstream_defaults_to_depth_one() {
    let it = wired();
    let out = it.w.run(&["docs", "deps", "ingest-api", "--downstream"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("sqs file-ingest -> record-store"), "{text}");
    assert!(!text.contains("report-builder"), "{text}");
    assert!(text.contains("1 repos, depth 1, truncated"), "{text}");
}

#[test]
fn s7_depth_two_reaches_the_second_hop() {
    let it = wired();
    let out =
        it.w.run(&["docs", "deps", "ingest-api", "--downstream", "--depth", "2"]);
    let text = stdout(&out);
    assert!(text.contains("record-store"), "{text}");
    assert!(text.contains("GET /records -> report-builder"), "{text}");
    assert!(text.contains("2 repos, depth 2"), "{text}");
}

#[test]
fn s7_depth_zero_is_unlimited() {
    let it = wired();
    let out =
        it.w.run(&["docs", "deps", "ingest-api", "--downstream", "--depth", "0"]);
    let text = stdout(&out);
    assert!(text.contains("report-builder"), "{text}");
    assert!(!text.contains("truncated"), "{text}");
}

#[test]
fn s7_upstream_walks_the_other_way() {
    let it = wired();
    let out = it.w.run(&[
        "docs",
        "deps",
        "report-builder",
        "--upstream",
        "--depth",
        "0",
    ]);
    let text = stdout(&out);
    assert!(text.contains("record-store"), "{text}");
    assert!(text.contains("ingest-api"), "{text}");
}

#[test]
fn s7_a_repo_without_edges_says_so() {
    let it = wired();
    let other = it.w.other_repo("lonely");
    it.w.write_docs_in(&other, &[("00-index.md", &index_page("2026-09-02"))]);
    it.w.commit_push_in(&other, "docs");
    assert!(it.w.run_in(&other, &["add"]).status.success());
    let out =
        it.w.run_in(&other, &["docs", "deps", "lonely", "--downstream"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("no edges declared for lonely"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn s7_a_direction_is_required() {
    let it = wired();
    let out = it.w.run(&["docs", "deps", "ingest-api"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
}

#[test]
fn s7_a_cycle_is_marked_once_and_terminates() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[
        ("00-index.md", &index_page("2026-09-04")),
        (
            "09-interfaces.md",
            &produces_page("2026-09-04", "loop-b", "sqs", "a-to-b"),
        ),
    ]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    let b = w.other_repo("loop-b");
    w.write_docs_in(
        &b,
        &[
            ("00-index.md", &index_page("2026-09-04")),
            (
                "09-interfaces.md",
                &produces_page("2026-09-04", "ingest-api", "sqs", "b-to-a"),
            ),
        ],
    );
    w.commit_push_in(&b, "docs");
    assert!(w.run_in(&b, &["add"]).status.success());
    assert!(w.run(&["sync"]).status.success());
    let out = w.run(&["docs", "deps", "ingest-api", "--downstream", "--depth", "0"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("(cycle)"), "{}", stdout(&out));
}

#[test]
fn s7_path_prints_the_chain() {
    let it = wired();
    let out = it.w.run(&["docs", "path", "ingest-api", "report-builder"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "ingest-api -[sqs file-ingest]-> record-store -[http GET /records]-> report-builder"
        ),
        "{text}"
    );
    assert!(text.contains("2 hops"), "{text}");
}

#[test]
fn s7_path_falls_back_to_the_reverse_direction() {
    let it = wired();
    let out = it.w.run(&["docs", "path", "report-builder", "ingest-api"]);
    let text = stdout(&out);
    assert!(text.contains("reverse path"), "{text}");
}

#[test]
fn s7_no_path_is_exit_zero() {
    let it = wired();
    let other = it.w.other_repo("island");
    it.w.write_docs_in(&other, &[("00-index.md", &index_page("2026-09-02"))]);
    it.w.commit_push_in(&other, "docs");
    assert!(it.w.run_in(&other, &["add"]).status.success());
    assert!(it.w.run(&["sync"]).status.success());
    let out = it.w.run(&["docs", "path", "ingest-api", "island"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("no path"), "{}", stdout(&out));
}

#[test]
fn s7_an_unknown_repo_refuses() {
    let it = wired();
    let out = it.w.run(&["docs", "deps", "ghost", "--downstream"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(
        stderr(&out).contains("unknown repo ghost"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn s7_a_second_contract_to_one_consumer_is_not_a_cycle() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    let data = w.other_repo("record-store");
    w.write_docs_in(
        &data,
        &[
            ("00-index.md", &index_page("2026-09-04")),
            (
                "09-interfaces.md",
                "---\ngenerated_date: 2026-09-04\nproduces:\n  - kind: http\n    name: GET /a\n    to: report-builder\n  - kind: http\n    name: GET /b\n    to: report-builder\n---\n\n## Produces\n\n| Kind | Name |\n|---|---|\n| http | GET /a |\n| http | GET /b |\n",
            ),
        ],
    );
    w.commit_push_in(&data, "docs");
    assert!(w.run_in(&data, &["add"]).status.success());
    let report_builder = w.other_repo("report-builder");
    w.write_docs_in(
        &report_builder,
        &[("00-index.md", &index_page("2026-09-01"))],
    );
    w.commit_push_in(&report_builder, "docs");
    assert!(w.run_in(&report_builder, &["add"]).status.success());
    assert!(w.run(&["sync"]).status.success());
    let out = w.run(&[
        "docs",
        "deps",
        "record-store",
        "--downstream",
        "--depth",
        "0",
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("http GET /a -> report-builder"), "{text}");
    assert!(text.contains("http GET /b -> report-builder"), "{text}");
    assert!(!text.contains("(cycle)"), "{text}");
    assert!(text.contains("1 repos, depth 1"), "{text}");
}
