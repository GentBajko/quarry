//! S3: root index regeneration.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;

#[test]
fn s3_the_root_index_lists_every_repo_with_its_counts() {
    let it = wired();
    let root = it.w.remote_file("00-index.md");
    assert!(root.starts_with("# docs-quarry\n"), "{root}");
    assert!(
        root.contains("| Repo | Imported | Newest doc | Pages | Produces | Consumes |"),
        "{root}"
    );
    let rows: Vec<&str> = root.lines().filter(|l| l.starts_with("| [")).collect();
    assert_eq!(rows.len(), 3, "{root}");
    assert!(
        rows[0].contains("[ingest-api](ingest-api/00-index.md)"),
        "{root}"
    );
    assert!(rows[1].contains("record-store"), "{root}");
    assert!(rows[2].contains("report-builder"), "{root}");
    assert!(rows[0].contains("2026-09-04"), "{root}");
}

#[test]
fn s3_regeneration_is_byte_identical_for_the_same_folder_state() {
    let it = wired();
    let first = it.w.remote_file("00-index.md");
    assert!(it.w.run(&["update"]).status.success());
    let second = it.w.remote_file("00-index.md");
    assert_eq!(first, second);
}

#[test]
fn s3_an_empty_docs_repo_gets_a_header_only_index() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[("00-index.md", &index_page("2026-09-04"))]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    assert!(w.run(&["remove"]).status.success());
    let root = w.remote_file("00-index.md");
    assert!(root.contains("| Repo |"), "{root}");
    assert!(!root.contains("| ["), "{root}");
}
