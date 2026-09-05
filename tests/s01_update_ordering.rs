//! S1: update ordering and concurrency.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;

fn added() -> World {
    let w = world();
    assert_eq!(code(&w.run(&["init", "--url", &w.docs_url])), 0);
    w.write_docs(&[("00-index.md", &index_page("2026-09-04"))]);
    w.commit_push("docs");
    assert_eq!(code(&w.run(&["add"])), 0);
    w
}

#[test]
fn s1_same_commit_is_current_and_writes_nothing() {
    let w = added();
    let before = stdout(&w.git(
        &w.source.join(".quarry/docs-quarry"),
        &["rev-parse", "HEAD"],
    ));
    let out = w.run(&["update"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("current"), "{}", stdout(&out));
    w.run(&["sync"]);
    let after = stdout(&w.git(
        &w.source.join(".quarry/docs-quarry"),
        &["rev-parse", "HEAD"],
    ));
    assert_eq!(before, after);
}

#[test]
fn s1_a_newer_commit_is_imported() {
    let w = added();
    w.write_docs(&[(
        "01-architecture.md",
        "---\ngenerated_date: 2026-09-05\n---\n\n# Layers\n\nx\n",
    )]);
    let head = w.commit_push("more docs");
    let out = w.run(&["update"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        w.remote_files()
            .contains(&"ingest-api/01-architecture.md".to_string())
    );
    let stamp = w.remote_file("ingest-api/.quarry-stamp");
    assert!(stamp.contains(&head), "{stamp}");
    assert!(
        stamp.contains("\"origin\":\"localhost/remotes/ingest-api\""),
        "{stamp}"
    );
}

#[test]
fn s1_never_go_backwards() {
    let w = added();
    w.write_docs(&[("01-architecture.md", "# later\n")]);
    w.commit_push("later");
    assert_eq!(code(&w.run(&["update"])), 0);
    let older = stdout(&w.git(&w.source, &["rev-parse", "HEAD~1"]))
        .trim()
        .to_string();
    w.git(&w.source, &["checkout", "--quiet", &older]);
    let out = w.run(&["update"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("newer commit, skipped"),
        "{}",
        stdout(&out)
    );
    assert!(
        w.remote_files()
            .contains(&"ingest-api/01-architecture.md".to_string())
    );
}

#[test]
fn s1_commit_not_on_the_default_branch_refuses() {
    let w = added();
    w.git(&w.source, &["checkout", "--quiet", "-b", "feature"]);
    w.write_docs(&[("02-models.md", "# branch only\n")]);
    w.git(&w.source, &["add", "-A"]);
    w.git(&w.source, &["commit", "--quiet", "-m", "branch work"]);
    let out = w.run(&["update"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stderr(&out).contains("merge first"), "{}", stderr(&out));
}

#[test]
fn s1_a_diverged_stamp_refuses_until_forced() {
    let w = added();
    w.write_docs(&[("01-architecture.md", "# first\n")]);
    w.commit_push("first");
    assert_eq!(code(&w.run(&["update"])), 0);
    w.write_docs(&[("01-architecture.md", "# rewritten\n")]);
    w.git(&w.source, &["add", "-A"]);
    w.git(
        &w.source,
        &["commit", "--quiet", "--amend", "-m", "rewritten"],
    );
    let pushed = w.git(&w.source, &["push", "--quiet", "--force", "origin", "main"]);
    assert!(pushed.status.success(), "{}", stderr(&pushed));
    let out = w.run(&["update"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stderr(&out).contains("diverged"), "{}", stderr(&out));
    let forced = w.run(&["update", "--force"]);
    assert_eq!(code(&forced), 0, "{}", stderr(&forced));
    assert!(
        w.remote_file("ingest-api/01-architecture.md")
            .contains("rewritten")
    );
}

#[test]
fn s1_an_unreachable_stamp_refuses_until_forced() {
    let w = added();
    let clone = w.docs_clone("tamper");
    std::fs::write(
        clone.join("ingest-api/.quarry-stamp"),
        "{\"commit\":\"0000000000000000000000000000000000000000\",\"origin\":\"localhost/remotes/ingest-api\"}\n",
    )
    .expect("write stamp");
    w.git(&clone, &["add", "-A"]);
    w.git(&clone, &["commit", "--quiet", "-m", "tamper"]);
    let pushed = w.git(&clone, &["push", "--quiet", "origin", "main"]);
    assert!(pushed.status.success(), "{}", stderr(&pushed));
    w.write_docs(&[("01-architecture.md", "# next\n")]);
    w.commit_push("next");
    let out = w.run(&["update"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stderr(&out).contains("not reachable"), "{}", stderr(&out));
    assert_eq!(code(&w.run(&["update", "--force"])), 0);
}

#[test]
fn s1_a_folder_without_a_stamp_is_imported_unconditionally() {
    let w = added();
    let clone = w.docs_clone("strip");
    std::fs::remove_file(clone.join("ingest-api/.quarry-stamp")).expect("remove stamp");
    w.git(&clone, &["add", "-A"]);
    w.git(&clone, &["commit", "--quiet", "-m", "strip stamp"]);
    assert!(
        w.git(&clone, &["push", "--quiet", "origin", "main"])
            .status
            .success()
    );
    let out = w.run(&["update"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("files imported"), "{}", stdout(&out));
}

#[test]
fn s1_a_stale_clone_is_reset_before_importing() {
    let w = added();
    let other = w.other_repo("identity-api");
    w.write_docs_in(&other, &[("00-index.md", &index_page("2026-09-02"))]);
    w.commit_push_in(&other, "docs");
    assert_eq!(code(&w.run_in(&other, &["add"])), 0);
    w.write_docs(&[("01-architecture.md", "# next\n")]);
    w.commit_push("next");
    let out = w.run(&["update"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let root = w.remote_file("00-index.md");
    assert!(root.contains("ingest-api"), "{root}");
    assert!(root.contains("identity-api"), "{root}");
}

#[test]
fn s1_a_rejected_push_is_redone_and_lands() {
    let w = added();
    let hook = w.base().join("remotes/docs-quarry.git/hooks/update");
    std::fs::write(
        &hook,
        "#!/bin/sh\nif [ ! -f \"$GIT_DIR/rejected-once\" ]; then\n  touch \"$GIT_DIR/rejected-once\"\n  echo 'rejected by test hook' >&2\n  exit 1\nfi\nexit 0\n",
    )
    .expect("write hook");
    let mut perms = std::fs::metadata(&hook).expect("meta").permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&hook, perms).expect("chmod");
    w.write_docs(&[("01-architecture.md", "# retried\n")]);
    w.commit_push("retried");
    let out = w.run(&["update"]);
    assert_eq!(code(&out), 0, "{}\n{}", stdout(&out), stderr(&out));
    assert!(
        w.remote_file("ingest-api/01-architecture.md")
            .contains("retried")
    );
}

#[test]
fn s1_a_second_repo_claiming_the_same_name_refuses() {
    let w = added();
    let base = w.base();
    std::fs::create_dir_all(base.join("other")).expect("dir");
    let clash = common::new_source(&base.join("other"), "ingest-api");
    let mirror = base.join("other/mirror");
    std::fs::create_dir_all(&mirror).expect("mirror dir");
    let mirror_url = format!("file://{}/ingest-api.git", mirror.to_string_lossy());
    w.git(
        &base,
        &[
            "init",
            "--quiet",
            "--bare",
            "--initial-branch=main",
            &format!("{}/ingest-api.git", mirror.to_string_lossy()),
        ],
    );
    w.git(&clash, &["remote", "set-url", "origin", &mirror_url]);
    w.git(&clash, &["push", "--quiet", "origin", "main"]);
    w.git(&clash, &["remote", "set-head", "origin", "-a"]);
    assert_eq!(code(&w.run_in(&clash, &["init", "--url", &w.docs_url])), 0);
    w.write_docs_in(&clash, &[("00-index.md", &index_page("2026-09-01"))]);
    w.commit_push_in(&clash, "docs");
    let out = w.run_in(&clash, &["add"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stderr(&out).contains("already used by"), "{}", stderr(&out));
}
