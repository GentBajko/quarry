//! S11: network and auth failure.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;

#[test]
fn s11_a_bad_docs_repo_url_exits_two() {
    let w = world();
    let out = w.run(&["init", "--url", "file:///nonexistent/docs.git"]);
    assert_eq!(code(&out), 2, "{}", stdout(&out));
    assert!(
        stderr(&out).contains("git clone failed"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn s11_a_vanished_remote_exits_two_on_update() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[("00-index.md", &index_page("2026-09-04"))]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    let remote = w.base().join("remotes/docs-quarry.git");
    std::fs::rename(&remote, w.base().join("remotes/gone.git")).expect("move remote");
    w.write_docs(&[("01-architecture.md", "# more\n")]);
    w.commit_push("more");
    let out = w.run(&["update"]);
    assert_eq!(code(&out), 2, "{}", stdout(&out));
}

#[test]
fn s11_a_diverged_clone_is_reset_and_reported() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[("00-index.md", &index_page("2026-09-04"))]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    let clone = w.source.join(".quarry/docs-quarry");
    std::fs::write(clone.join("hand-edit.md"), "# by hand\n").expect("hand edit");
    w.git(&clone, &["add", "-A"]);
    w.git(&clone, &["commit", "--quiet", "-m", "hand edit"]);
    let out = w.run(&["sync"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("discarded 1 local commits"),
        "{}",
        stdout(&out)
    );
    assert!(!clone.join("hand-edit.md").exists());
}

#[test]
fn s11_an_uncommitted_edit_in_the_clone_is_discarded() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    w.write_docs(&[("00-index.md", &index_page("2026-09-04"))]);
    w.commit_push("docs");
    assert!(w.run(&["add"]).status.success());
    let clone = w.source.join(".quarry/docs-quarry");
    std::fs::write(clone.join("ingest-api/00-index.md"), "clobbered\n").expect("edit");
    assert!(w.run(&["sync"]).status.success());
    let restored = std::fs::read_to_string(clone.join("ingest-api/00-index.md")).expect("read");
    assert!(restored.contains("# Overview"), "{restored}");
}
