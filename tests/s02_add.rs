//! S2: add / remove lifecycle, and S3's root index on the first import.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;

fn initialised() -> World {
    let w = world();
    let out = w.run(&["init", "--url", &w.docs_url]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    w.write_docs(&[("00-index.md", &index_page("2026-09-04"))]);
    w.commit_push("docs");
    w
}

#[test]
fn s2_add_first_import_lands_in_the_docs_repo() {
    let w = initialised();
    let out = w.run(&["add"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let files = w.remote_files();
    assert!(
        files.contains(&"ingest-api/00-index.md".to_string()),
        "{files:?}"
    );
    assert!(
        files.contains(&"ingest-api/.quarry-stamp".to_string()),
        "{files:?}"
    );
    assert!(files.contains(&"00-index.md".to_string()), "{files:?}");
}

#[test]
fn s2_add_twice_is_idempotent() {
    let w = initialised();
    w.run(&["add"]);
    let out = w.run(&["add"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("already in the docs repo"),
        "{}",
        stdout(&out)
    );
    assert!(stdout(&out).contains("current"), "{}", stdout(&out));
}

#[test]
fn s2_update_before_add_refuses() {
    let w = initialised();
    let out = w.run(&["update"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stderr(&out).contains("run quarry add"), "{}", stderr(&out));
}

#[test]
fn s2_add_without_an_index_page_refuses() {
    let w = world();
    w.run(&["init", "--url", &w.docs_url]);
    w.write_docs(&[("01-architecture.md", "# no index\n")]);
    w.commit_push("docs");
    let out = w.run(&["add"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stderr(&out).contains("no 00-index.md"), "{}", stderr(&out));
}

#[test]
fn s2_add_without_a_docs_folder_refuses() {
    let w = world();
    w.run(&["init", "--url", &w.docs_url]);
    let out = w.run(&["add"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stderr(&out).contains("no 00-index.md"), "{}", stderr(&out));
}

#[test]
fn s2_remove_when_absent_is_a_no_op() {
    let w = initialised();
    let out = w.run(&["remove"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("is not in the docs repo"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn s2_remove_drops_the_folder_and_the_index_row() {
    let w = initialised();
    w.run(&["add"]);
    let out = w.run(&["remove"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let files = w.remote_files();
    assert!(
        !files.iter().any(|f| f.starts_with("ingest-api/")),
        "{files:?}"
    );
    let root = w.remote_file("00-index.md");
    assert!(!root.contains("ingest-api"), "{root}");
}

#[test]
fn s2_the_working_repo_keeps_its_quarry_dir_after_remove() {
    let w = initialised();
    w.run(&["add"]);
    w.run(&["remove"]);
    assert!(w.source.join(".quarry/.config").exists());
}
