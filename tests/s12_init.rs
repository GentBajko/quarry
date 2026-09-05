//! S12: init and configuration.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;

#[test]
fn s12_init_writes_config_gitignore_and_clone() {
    let w = world();
    let out = w.run(&["init", "--url", &w.docs_url]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let config = std::fs::read_to_string(w.source.join(".quarry/.config")).expect("config");
    assert!(config.contains(&w.docs_url), "{config}");
    assert!(
        config.contains("\"docs_dir\": \"docs/capstone\""),
        "{config}"
    );
    assert!(config.contains("\"default_branch\": \"main\""), "{config}");
    let ignore = std::fs::read_to_string(w.source.join(".quarry/.gitignore")).expect("gitignore");
    assert!(ignore.contains("docs-quarry/"), "{ignore}");
    assert!(ignore.contains(".docs-index.sqlite"), "{ignore}");
    assert!(w.source.join(".quarry/docs-quarry/.git").exists());
    assert!(
        stdout(&out).contains("this repo is `ingest-api`"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn s12_the_clone_is_ignored_by_the_source_repo() {
    let w = world();
    w.run(&["init", "--url", &w.docs_url]);
    let status = stdout(&w.git(&w.source, &["status", "--porcelain"]));
    assert!(!status.contains("docs-quarry"), "{status}");
}

#[test]
fn s12_rerun_with_the_same_url_is_a_no_op() {
    let w = world();
    w.run(&["init", "--url", &w.docs_url]);
    let out = w.run(&["init", "--url", &w.docs_url]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("clone current"), "{}", stdout(&out));
}

#[test]
fn s12_a_different_url_refuses_without_force() {
    let w = world();
    w.run(&["init", "--url", &w.docs_url]);
    let other = format!("{}x", w.docs_url);
    let out = w.run(&["init", "--url", &other]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(
        stderr(&out).contains("already linked to"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn s12_force_relinks_and_replaces_the_clone() {
    let w = world();
    w.run(&["init", "--url", &w.docs_url]);
    let base = w.base();
    let second = format!(
        "file://{}",
        base.join("remotes/other-docs.git").to_string_lossy()
    );
    w.git(
        &base,
        &[
            "init",
            "--quiet",
            "--bare",
            "--initial-branch=main",
            &base.join("remotes/other-docs.git").to_string_lossy(),
        ],
    );
    let out = w.run(&["init", "--url", &second, "--force"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(w.source.join(".quarry/other-docs/.git").exists());
    assert!(!w.source.join(".quarry/docs-quarry").exists());
}

#[test]
fn s12_no_url_and_no_terminal_refuses_naming_the_flags() {
    let w = world();
    let out = w.run(&["init"]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stderr(&out).contains("--url"), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("QUARRY_DOCS_REPO"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn s12_env_var_stands_in_for_the_flag() {
    let w = world();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_quarry"))
        .args(["init"])
        .current_dir(&w.source)
        .env("QUARRY_DOCS_REPO", &w.docs_url)
        .env("GIT_AUTHOR_NAME", "quarry test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "quarry test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .env("GIT_CONFIG_GLOBAL", w.base().join("gitconfig"))
        .env("GIT_CONFIG_SYSTEM", w.base().join("gitconfig"))
        .output()
        .expect("run");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(w.source.join(".quarry/.config").exists());
}

#[test]
fn s12_missing_docs_folder_warns_but_still_writes_config() {
    let w = world();
    let out = w.run(&["init", "--url", &w.docs_url]);
    assert!(
        stdout(&out).contains("no docs/capstone yet"),
        "{}",
        stdout(&out)
    );
    assert!(w.source.join(".quarry/.config").exists());
}

#[test]
fn s12_outside_a_git_repo_refuses() {
    let w = world();
    let outside = w.base().join("elsewhere");
    std::fs::create_dir_all(&outside).expect("dir");
    let out = w.run_in(&outside, &["init", "--url", &w.docs_url]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(
        stderr(&out).contains("not a git repository"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn s12_a_fresh_clone_is_not_reported_as_never_synced() {
    let w = world();
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    let out = w.run(&["docs", "list"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        !stdout(&out).contains("never synced"),
        "init cloned from the remote, so nothing is stale: {}",
        stdout(&out)
    );
}

#[test]
fn s12_a_greenfield_repo_can_read_the_quarry_before_it_contributes() {
    let w = world();
    w.write_docs(&[("00-index.md", &index_page("2026-09-04"))]);
    w.commit_push("docs");
    assert!(w.run(&["init", "--url", &w.docs_url]).status.success());
    assert!(w.run(&["add"]).status.success());

    let base = w.base();
    let blank = base.join("greenfield");
    std::fs::create_dir_all(&blank).expect("dir");
    w.git(
        &base,
        &[
            "init",
            "--quiet",
            "--initial-branch=main",
            &blank.to_string_lossy(),
        ],
    );
    let init = w.run_in(&blank, &["init", "--url", &w.docs_url]);
    assert_eq!(code(&init), 0, "{}", stderr(&init));

    let list = w.run_in(&blank, &["docs", "list"]);
    assert_eq!(code(&list), 0, "{}", stderr(&list));
    assert!(stdout(&list).contains("ingest-api"), "{}", stdout(&list));
}
