//! S10: removal with dangling edges.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;

#[test]
fn s10_remove_reports_the_repos_that_still_point_here() {
    let it = wired();
    let out = it.w.run_in(&it.data, &["remove"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("removed record-store"), "{text}");
    assert!(
        text.contains("still declare edges to record-store"),
        "{text}"
    );
    assert!(text.contains("ingest-api"), "{text}");
    assert!(text.contains("report-builder"), "{text}");
}

#[test]
fn s10_the_edges_become_dangling_but_stay_visible() {
    let it = wired();
    assert!(it.w.run_in(&it.data, &["remove"]).status.success());
    assert!(it.w.run(&["sync"]).status.success());
    let out = it.w.run(&["docs", "deps", "ingest-api", "--downstream"]);
    let text = stdout(&out);
    assert!(text.contains("record-store"), "{text}");
    assert!(text.contains("(not in quarry)"), "{text}");
}

#[test]
fn s10_re_adding_resolves_the_edges_again() {
    let it = wired();
    assert!(it.w.run_in(&it.data, &["remove"]).status.success());
    assert!(it.w.run_in(&it.data, &["add"]).status.success());
    assert!(it.w.run(&["sync"]).status.success());
    let out = it.w.run(&["docs", "deps", "ingest-api", "--downstream"]);
    assert!(
        !stdout(&out).contains("(not in quarry)"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn s10_removing_a_repo_nobody_references_reports_nothing() {
    let it = wired();
    let other = it.w.other_repo("standalone");
    it.w.write_docs_in(&other, &[("00-index.md", &index_page("2026-09-02"))]);
    it.w.commit_push_in(&other, "docs");
    assert!(it.w.run_in(&other, &["add"]).status.success());
    let out = it.w.run_in(&other, &["remove"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(!stdout(&out).contains("still declare"), "{}", stdout(&out));
}
